import SwiftUI

struct ChatView: View {
    @EnvironmentObject private var session: SessionViewModel

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 8) {
                            ForEach(session.chatLines) { line in
                                Text(line.text)
                                    .textSelection(.enabled)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .id(line.id)
                            }
                        }
                        .padding()
                    }
                    .onChange(of: session.chatLines.count) { _, _ in
                        if let last = session.chatLines.last {
                            withAnimation {
                                proxy.scrollTo(last.id, anchor: .bottom)
                            }
                        }
                    }
                }

                Divider()

                HStack(spacing: 8) {
                    TextField("Message", text: $session.draft)
                        .textFieldStyle(.roundedBorder)
                        .disabled(!session.canSend)
                        .onSubmit { session.sendMessage() }
                        .accessibilityLabel("Message")
                        .accessibilityIdentifier("chatField")

                    Button("Send") {
                        session.sendMessage()
                    }
                    .disabled(!session.canSend || session.draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("sendButton")
                }
                .padding()
            }
            .navigationTitle(session.selectedCharacter?.name ?? "Chat")
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
