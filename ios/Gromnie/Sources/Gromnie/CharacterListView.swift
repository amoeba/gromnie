import GromnieKit
import SwiftUI

struct CharacterListView: View {
    @EnvironmentObject private var session: SessionViewModel

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if case .error(let message) = session.status {
                    CharacterLoginErrorBanner(message: message) {
                        session.disconnect()
                    }
                }
                List(session.characters) { character in
                    Button {
                        session.select(character)
                    } label: {
                        Text(character.name)
                    }
                    .accessibilityIdentifier("character-\(character.id)")
                }
                .overlay {
                    if session.characters.isEmpty {
                        ContentUnavailableView("No characters", systemImage: "person.crop.circle.badge.questionmark")
                    }
                }
            }
            .navigationTitle("Characters")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Disconnect") {
                        session.disconnect()
                    }
                    .accessibilityIdentifier("disconnectButton")
                }
            }
        }
    }
}

/// Shows the reason a character login was rejected, mirroring the TUI's error
/// view, and returns the user to the server select screen to recover.
private struct CharacterLoginErrorBanner: View {
    let message: String
    let onReturnToServerSelect: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Error", systemImage: "exclamationmark.triangle.fill")
                .font(.headline)
                .foregroundStyle(.red)
            Text(message)
                .font(.footnote)
                .foregroundStyle(.secondary)
            HStack {
                Spacer()
                Button("Back to server select", action: onReturnToServerSelect)
                    .accessibilityIdentifier("backToServerSelectButton")
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.red.opacity(0.1))
        .accessibilityIdentifier("loginErrorBanner")
    }
}
