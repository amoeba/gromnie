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
            ProgressView("Entering world…")
        case .chat:
            ChatView()
        }
    }
}