import SwiftUI

@main
struct GromnieApp: App {
    @StateObject private var session = SessionViewModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(session)
        }
        .onChange(of: scenePhase) { _, phase in
            session.handleScenePhase(phase)
        }
    }
}