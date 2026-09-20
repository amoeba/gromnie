import Foundation
import GromnieCore

/// Errors returned by the Rust bridge's C ABI.
public enum GromnieCoreError: Error, LocalizedError, Equatable {
    case noEvent
    case invalidArgument
    case invalidState
    case queueFull
    case timedOut
    case internalError
    case unknown(Int32)

    init(code: Int32) {
        switch code {
        case 1: self = .noEvent
        case 2: self = .invalidArgument
        case 3: self = .invalidState
        case 4: self = .queueFull
        case 5: self = .timedOut
        case 6: self = .internalError
        default: self = .unknown(code)
        }
    }

    public var errorDescription: String? {
        switch self {
        case .noEvent: return "No event was available."
        case .invalidArgument: return "The bridge rejected the input."
        case .invalidState: return "The session is not in a valid state."
        case .queueFull: return "The bridge command queue is full."
        case .timedOut: return "The bridge operation timed out."
        case .internalError: return "The bridge encountered an internal error."
        case .unknown(let code): return "The bridge returned an unknown error (\(code))."
        }
    }
}

/// Owns the opaque Rust session. Every C call runs on one serial queue, and
/// events are delivered through an `AsyncStream`. No Rust callback ever invokes
/// Swift directly.
public final class GromnieCoreClient {
    private let queue = DispatchQueue(label: "net.gromnie.core")
    private var session: OpaquePointer?
    private var continuation: AsyncStream<BridgeEvent>.Continuation?
    private var isPolling = false

    public init() {}

    deinit {
        // Best-effort: `disconnect()` should already have run. This exists so a
        // dropped client never leaves a Rust actor behind.
        guard let session else { return }
        self.session = nil
        queue.async {
            _ = gromnie_session_disconnect(session)
            gromnie_session_destroy(session)
        }
    }

    /// Starts a direct-UDP login and returns the event stream. Network outcomes
    /// arrive as events; this only throws for local API errors.
    public func connect(
        host: String,
        port: UInt16,
        username: String,
        password: String
    ) throws -> AsyncStream<BridgeEvent> {
        try queue.sync {
            destroyLocked()
            guard let session = gromnie_session_create() else {
                throw GromnieCoreError.internalError
            }
            self.session = session

            let result = host.withCString { hostPointer in
                username.withCString { usernamePointer in
                    password.withCString { passwordPointer in
                        gromnie_session_connect(
                            session,
                            hostPointer,
                            port,
                            usernamePointer,
                            passwordPointer
                        )
                    }
                }
            }

            guard result == 0 else {
                destroyLocked()
                throw GromnieCoreError(code: result)
            }
        }

        let stream = AsyncStream<BridgeEvent> { continuation in
            self.continuation = continuation
        }
        startPolling()
        return stream
    }

    /// Requests character selection. Progress is reported through the event stream.
    public func selectCharacter(id: UInt32) {
        queue.async {
            guard let session = self.session else { return }
            _ = gromnie_session_select_character(session, id)
        }
    }

    /// Sends a chat line. Returns `true` when Rust accepted it into its command queue.
    public func sendChat(_ message: String) async -> Bool {
        await withCheckedContinuation { continuation in
            queue.async {
                guard let session = self.session else {
                    continuation.resume(returning: false)
                    return
                }
                let result = message.withCString { pointer in
                    gromnie_session_send_chat(session, pointer)
                }
                continuation.resume(returning: result == 0)
            }
        }
    }

    /// Disconnects, joins the Rust actor, and frees the session.
    public func disconnect() {
        queue.async {
            self.isPolling = false
            if let session = self.session {
                _ = gromnie_session_disconnect(session)
            }
            self.destroyLocked()
            self.continuation?.finish()
        }
    }

    // MARK: - Queue-confined helpers

    private func startPolling() {
        queue.async {
            self.isPolling = true
            self.schedulePoll()
        }
    }

    /// Schedules one poll iteration as its own queue block so queued commands
    /// get a chance to run between the (up to 250 ms) blocking C calls.
    private func schedulePoll() {
        queue.async { [weak self] in
            guard let self, self.isPolling, let session = self.session else { return }

            var jsonPointer: UnsafeMutablePointer<UInt8>?
            var jsonLength = 0
            let result = gromnie_session_next_event(session, 250, &jsonPointer, &jsonLength)

            if result == 0, let jsonPointer {
                defer { gromnie_buffer_free(jsonPointer, jsonLength) }
                let data = Data(bytes: jsonPointer, count: jsonLength)
                if let event = try? JSONDecoder().decode(BridgeEvent.self, from: data) {
                    self.continuation?.yield(event)
                    if event.type == BridgeEventType.disconnected.rawValue {
                        self.isPolling = false
                        self.continuation?.finish()
                        return
                    }
                }
            } else if result != 1 {
                // Anything other than `no_event` means the session is unusable.
                self.isPolling = false
                self.continuation?.finish()
                return
            }

            guard self.isPolling else { return }
            self.schedulePoll()
        }
    }

    private func destroyLocked() {
        guard let session else { return }
        self.session = nil
        gromnie_session_destroy(session)
    }
}