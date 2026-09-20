import SwiftUI

struct RootView: View {
    @EnvironmentObject private var session: SessionViewModel

    var body: some View {
        switch session.screen {
        case .form:
            ConnectionView()
        case .characters:
            CharacterListView()
        case .enteringWorld:
            VStack(spacing: 16) {
                ProgressView("Entering world…")
                Button("Disconnect") {
                    session.disconnect()
                }
                .accessibilityIdentifier("disconnectButton")
            }
        case .chat:
            ChatView()
        }
    }
}
