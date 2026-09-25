import GromnieKit
import XCTest

@testable import Gromnie

@MainActor
final class SessionViewModelTests: XCTestCase {
    private func decode(_ json: String) -> BridgeEvent {
        // Tests deliberately fail loudly if the contract is malformed.
        try! JSONDecoder().decode(BridgeEvent.self, from: Data(json.utf8))
    }

    private func chatEvent(_ message: String) -> BridgeEvent {
        decode(
            #"{"sequence":1,"timestamp_ms":1,"type":"chat","message":"\#(message)","message_type":1}"#
        )
    }

    func testHostValidationAcceptsHostnameAndIPv4() {
        XCTAssertTrue(SessionViewModel.isValidHost("play.example.com"))
        XCTAssertTrue(SessionViewModel.isValidHost("192.0.2.42"))
        XCTAssertFalse(SessionViewModel.isValidHost("https://play.example.com"))
        XCTAssertFalse(SessionViewModel.isValidHost("[2001:db8::1]"))
        XCTAssertFalse(SessionViewModel.isValidHost("bad host"))
        XCTAssertFalse(SessionViewModel.isValidHost("host:9000"))
        XCTAssertFalse(SessionViewModel.isValidHost(""))
    }

    func testCharacterListEventMovesToCharacterScreen() {
        let viewModel = SessionViewModel()

        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"characters","account":"acct","slots":1,"characters":[{"id":5,"name":"Bob"}]}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .characters)
        XCTAssertEqual(viewModel.characters, [CharacterInfo(id: 5, name: "Bob")])
        XCTAssertEqual(viewModel.status, .idle)
    }

    func testEnteredWorldMovesToChatAndEnablesComposer() {
        let viewModel = SessionViewModel()

        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"entered_world","character_id":5,"character_name":"Bob"}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .chat)
        XCTAssertTrue(viewModel.canSend)
    }

    func testErrorRecoveryTargetsTheRightScreen() {
        let toForm = SessionViewModel()
        toForm.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"error","code":"authentication","message":"bad","recover_to":"form"}"#
            )
        )
        XCTAssertEqual(toForm.screen, .form)
        XCTAssertEqual(toForm.status, .error("bad"))
        XCTAssertFalse(toForm.canSend)

        let toCharacters = SessionViewModel()
        toCharacters.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"error","code":"login","message":"nope","recover_to":"characters"}"#
            )
        )
        XCTAssertEqual(toCharacters.screen, .characters)
    }

    func testLoginFailureMessageSurvivesTheTerminalDisconnect() {
        // The bridge always ends a failed login with a terminal disconnect
        // event. That generic transport reason must not erase the specific
        // failure message shown on the server select screen.
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"error","code":"authentication","message":"Connection timeout - server not responding","recover_to":"form"}"#
            )
        )
        viewModel.handle(
            decode(
                #"{"sequence":2,"timestamp_ms":2,"type":"disconnected","reason":"connection timed out","user_initiated":false}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertEqual(
            viewModel.status,
            .error("Connection timeout - server not responding")
        )
    }

    func testCharacterLoginFailureKeepsMessageOnCharactersScreen() {
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"characters","account":"acct","slots":1,"characters":[{"id":5,"name":"Bob"}]}"#
            )
        )
        viewModel.handle(
            decode(
                #"{"sequence":2,"timestamp_ms":2,"type":"error","code":"login","message":"EnterGameCharacterLocked","recover_to":"characters"}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .characters)
        XCTAssertEqual(viewModel.status, .error("EnterGameCharacterLocked"))
    }

    func testDisconnectFromACharacterErrorKeepsTheFailureMessage() {
        // Returning to the server select after a rejected character login
        // must keep the failure message so the user knows what to fix.
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"error","code":"character","message":"LogonServerFull","recover_to":"characters"}"#
            )
        )
        viewModel.disconnect()

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertEqual(viewModel.status, .error("LogonServerFull"))
    }

    func testDisconnectWithoutAPendingErrorReportsDisconnected() {
        let viewModel = SessionViewModel()
        viewModel.disconnect()

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertEqual(viewModel.status, .disconnected("Disconnected"))
    }

    func testCancellingWhileConnectingReturnsToForm() {
        // The Cancel button during the handshake must tear the session down
        // and return to the server form instead of pinning "Connecting…".
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(#"{"sequence":1,"timestamp_ms":1,"type":"connecting"}"#)
        )
        XCTAssertTrue(viewModel.isConnecting)

        viewModel.disconnect()

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertEqual(viewModel.status, .disconnected("Disconnected"))
        XCTAssertFalse(viewModel.isConnecting)
    }

    func testRejectedPasswordShowsServerReasonAndReturnsToForm() {
        // The server rejects a wrong password with LoginAccountBooted; the
        // bridge forwards it as an authentication error and then ends the
        // session. The server's reason must survive the terminal disconnect.
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"error","code":"authentication","message":"because the password entered for this account was not correct","recover_to":"form"}"#
            )
        )
        viewModel.handle(
            decode(
                #"{"sequence":2,"timestamp_ms":2,"type":"disconnected","reason":"authentication failed","user_initiated":false}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertEqual(
            viewModel.status,
            .error("because the password entered for this account was not correct")
        )
    }

    func testChatTranscriptIsBounded() {
        let viewModel = SessionViewModel()

        for index in 0..<1_050 {
            viewModel.handle(chatEvent("m\(index)"))
        }

        XCTAssertLessThanOrEqual(viewModel.chatLines.count, 1_000)
        XCTAssertEqual(viewModel.chatLines.first?.text, "m100")
        XCTAssertEqual(viewModel.chatLines.last?.text, "m1049")
    }

    func testDisconnectedEventResetsState() {
        let viewModel = SessionViewModel()
        viewModel.handle(
            decode(
                #"{"sequence":1,"timestamp_ms":1,"type":"characters","account":"acct","slots":1,"characters":[{"id":5,"name":"Bob"}]}"#
            )
        )
        viewModel.handle(chatEvent("hello"))

        viewModel.handle(
            decode(
                #"{"sequence":2,"timestamp_ms":2,"type":"disconnected","reason":"lost","user_initiated":false}"#
            )
        )

        XCTAssertEqual(viewModel.screen, .form)
        XCTAssertTrue(viewModel.characters.isEmpty)
        XCTAssertTrue(viewModel.chatLines.isEmpty)
        XCTAssertFalse(viewModel.canSend)
    }
}
