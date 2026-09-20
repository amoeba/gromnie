import GromnieKit
import XCTest

/// Decoding tests for the JSON contract between the Rust bridge and Swift.
/// These guard the wire format independently of the Rust serialization tests.
final class BridgeEventDecodingTests: XCTestCase {
    private func decode(_ json: String) throws -> BridgeEvent {
        try JSONDecoder().decode(BridgeEvent.self, from: Data(json.utf8))
    }

    func testDecodesCharactersEvent() throws {
        let event = try decode(
            #"{"sequence":3,"timestamp_ms":1700000000000,"type":"characters","account":"acct","slots":2,"characters":[{"id":1,"name":"Alice"}]}"#
        )

        XCTAssertEqual(event.sequence, 3)
        XCTAssertEqual(event.timestampMs, 1_700_000_000_000)
        XCTAssertEqual(event.type, "characters")
        XCTAssertEqual(event.account, "acct")
        XCTAssertEqual(event.slots, 2)
        XCTAssertEqual(event.characters, [CharacterInfo(id: 1, name: "Alice")])
    }

    func testDecodesChatEvent() throws {
        let event = try decode(
            #"{"sequence":4,"timestamp_ms":1700000000001,"type":"chat","message":"hello","message_type":2}"#
        )

        XCTAssertEqual(event.type, "chat")
        XCTAssertEqual(event.message, "hello")
        XCTAssertEqual(event.messageType, 2)
    }

    func testDecodesEnteringAndEnteredWorldEvents() throws {
        let entering = try decode(
            #"{"sequence":1,"timestamp_ms":1,"type":"entering_world","character_id":9}"#
        )
        XCTAssertEqual(entering.characterId, 9)

        let entered = try decode(
            #"{"sequence":2,"timestamp_ms":2,"type":"entered_world","character_id":9,"character_name":"Alice"}"#
        )
        XCTAssertEqual(entered.characterId, 9)
        XCTAssertEqual(entered.characterName, "Alice")
    }

    func testDecodesErrorEvent() throws {
        let event = try decode(
            #"{"sequence":5,"timestamp_ms":1700000000002,"type":"error","code":"login","message":"nope","recover_to":"characters"}"#
        )

        XCTAssertEqual(event.type, "error")
        XCTAssertEqual(event.code, "login")
        XCTAssertEqual(event.message, "nope")
        XCTAssertEqual(event.recoverTo, "characters")
    }

    func testDecodesDisconnectedEvent() throws {
        let event = try decode(
            #"{"sequence":6,"timestamp_ms":1700000000003,"type":"disconnected","reason":"done","user_initiated":true}"#
        )

        XCTAssertEqual(event.type, "disconnected")
        XCTAssertEqual(event.reason, "done")
        XCTAssertEqual(event.userInitiated, true)
    }
}
