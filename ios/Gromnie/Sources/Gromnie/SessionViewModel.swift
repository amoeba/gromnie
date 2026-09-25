import Foundation
import GromnieKit
import SwiftUI

@MainActor
final class SessionViewModel: ObservableObject {
    enum Screen: Equatable {
        case form
        case characters
        case enteringWorld
        case chat
    }

    enum ConnectionStatus: Equatable {
        case idle
        case connecting
        case error(String)
        case disconnected(String)
    }

    struct ChatLine: Identifiable, Equatable {
        let id = UUID()
        let text: String
        let isError: Bool
    }

    @Published var screen: Screen = .form
    @Published var status: ConnectionStatus = .idle

    @Published var host: String = ""
    @Published var port: String = "9000"
    @Published var username: String = ""
    @Published var password: String = ""
    @Published var savePassword: Bool = false

    @Published var characters: [CharacterInfo] = []
    @Published var selectedCharacter: CharacterInfo?
    @Published var chatLines: [ChatLine] = []
    @Published var draft: String = ""
    @Published var canSend: Bool = false

    private let client = GromnieCoreClient()
    private let credentials = CredentialsStore()
    private var eventTask: Task<Void, Never>?

    init() {
        host = credentials.host
        port = credentials.port
        username = credentials.username
        savePassword = credentials.savePassword
        password = credentials.loadPassword(host: host, port: port, username: username)
    }

    var isConnecting: Bool { status == .connecting }

    // MARK: - Actions

    func connect() {
        let host = host.trimmingCharacters(in: .whitespaces)
        guard Self.isValidHost(host) else {
            status = .error("Enter a hostname or IPv4 address (no scheme or port).")
            return
        }
        guard let portValue = UInt16(port), portValue >= 1 else {
            status = .error("Port must be between 1 and 65535.")
            return
        }
        guard !username.isEmpty else {
            status = .error("Enter an account name.")
            return
        }
        guard !password.isEmpty else {
            status = .error("Enter a password.")
            return
        }

        credentials.save(
            host: host,
            port: port,
            username: username,
            password: password,
            savePassword: savePassword
        )

        do {
            let stream = try client.connect(
                host: host,
                port: portValue,
                username: username,
                password: password
            )
            status = .connecting
            eventTask?.cancel()
            eventTask = Task { [weak self] in
                for await event in stream {
                    guard let self else { return }
                    self.handle(event)
                }
            }
        } catch {
            status = .error(error.localizedDescription)
        }
    }

    func select(_ character: CharacterInfo) {
        guard screen == .characters else { return }
        selectedCharacter = character
        screen = .enteringWorld
        client.selectCharacter(id: character.id)
    }

    func sendMessage() {
        let message = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard canSend, !message.isEmpty else { return }
        Task {
            let accepted = await client.sendChat(message)
            if accepted {
                draft = ""
            }
        }
    }

    func disconnect() {
        client.disconnect()
        eventTask?.cancel()
        eventTask = nil
        reset()
        screen = .form
        preservePendingErrorOrSetDisconnected("Disconnected")
    }

    func handleScenePhase(_ phase: ScenePhase) {
        // `.inactive` fires transiently (Control Center, notification shade,
        // incoming call), so it must not tear down an active session. Only a
        // real background transition disconnects; v1 has no background mode.
        guard phase == .background else { return }
        guard screen != .form || status == .connecting else { return }
        disconnect()
    }

    // MARK: - Event handling

    func handle(_ event: BridgeEvent) {
        switch BridgeEventType(rawValue: event.type) {
        case .connecting:
            status = .connecting

        case .characters:
            characters = event.characters ?? []
            status = .idle
            screen = .characters

        case .enteringWorld:
            screen = .enteringWorld

        case .enteredWorld:
            canSend = true
            screen = .chat

        case .chat:
            appendChat(event.message ?? "")

        case .error:
            status = .error(event.message ?? "Unknown error.")
            canSend = false
            if event.recoverTo == "form" {
                screen = .form
            } else {
                screen = .characters
            }

        case .disconnected:
            // The actor always follows a fatal error (bad credentials, a
            // rejected character login, a lost network) with a terminal
            // disconnect event. Keep the specific failure message instead of
            // replacing it with the generic transport reason, so the server
            // select screen still explains why the session ended.
            preservePendingErrorOrSetDisconnected(event.reason ?? "Disconnected.")
            eventTask = nil
            reset()
            screen = .form

        case nil:
            break
        }
    }

    /// Sets a `.disconnected` status only when no failure message is already
    /// waiting to be shown. Returning to the server select after a failed
    /// login should keep the error visible so the user knows what to fix.
    private func preservePendingErrorOrSetDisconnected(_ reason: String) {
        if case .error = status {
            return
        }
        status = .disconnected(reason)
    }

    private func appendChat(_ message: String) {
        chatLines.append(ChatLine(text: message, isError: false))
        // Keep the transcript bounded: drop the oldest 100 once we hit 1,000.
        if chatLines.count > 1_000 {
            chatLines.removeFirst(100)
        }
    }

    private func reset() {
        characters = []
        selectedCharacter = nil
        chatLines = []
        draft = ""
        canSend = false
    }

    // MARK: - Validation

    static func isValidHost(_ host: String) -> Bool {
        guard !host.isEmpty else { return false }
        let invalid = CharacterSet(charactersIn: ":/[] ")
        guard host.rangeOfCharacter(from: invalid) == nil else { return false }
        return host.allSatisfy { character in
            (character.isASCII && (character.isLetter || character.isNumber))
                || character == "." || character == "-"
        }
    }
}
