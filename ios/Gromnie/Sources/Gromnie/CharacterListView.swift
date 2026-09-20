import GromnieKit
import SwiftUI

struct CharacterListView: View {
    @EnvironmentObject private var session: SessionViewModel

    var body: some View {
        NavigationStack {
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
