import Foundation

/// A character entry from the server's character list.
public struct CharacterInfo: Codable, Equatable, Identifiable, Sendable {
    public let id: UInt32
    public let name: String

    public init(id: UInt32, name: String) {
        self.id = id
        self.name = name
    }
}

/// A single event surfaced by the Rust bridge, decoded from the JSON returned
/// by `gromnie_session_next_event`.
public struct BridgeEvent: Codable, Equatable, Sendable {
    public let sequence: UInt64
    public let timestampMs: UInt64
    public let type: String

    // characters
    public let account: String?
    public let slots: UInt32?
    public let characters: [CharacterInfo]?

    // entering_world / entered_world
    public let characterId: UInt32?
    public let characterName: String?

    // chat
    public let message: String?
    public let messageType: UInt32?

    // error
    public let code: String?
    public let recoverTo: String?

    // disconnected
    public let reason: String?
    public let userInitiated: Bool?

    enum CodingKeys: String, CodingKey {
        case sequence
        case timestampMs = "timestamp_ms"
        case type
        case account
        case slots
        case characters
        case characterId = "character_id"
        case characterName = "character_name"
        case message
        case messageType = "message_type"
        case code
        case recoverTo = "recover_to"
        case reason
        case userInitiated = "user_initiated"
    }
}

/// The fixed set of `type` values emitted by the Rust bridge.
public enum BridgeEventType: String {
    case connecting
    case characters
    case enteringWorld = "entering_world"
    case enteredWorld = "entered_world"
    case chat
    case error
    case disconnected
}