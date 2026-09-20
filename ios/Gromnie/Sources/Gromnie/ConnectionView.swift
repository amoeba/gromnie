import SwiftUI

struct ConnectionView: View {
    @EnvironmentObject private var session: SessionViewModel

    var body: some View {
        NavigationStack {
            Form {
                Section("Server") {
                    TextField("Hostname or IPv4", text: $session.host)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityLabel("Hostname or IPv4 address")

                    TextField("Port", text: $session.port)
                        .keyboardType(.numberPad)
                        .accessibilityLabel("Port")
                }

                Section("Account") {
                    TextField("Account name", text: $session.username)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityLabel("Account name")

                    SecureField("Password", text: $session.password)
                        .accessibilityLabel("Password")

                    Toggle("Save password", isOn: $session.savePassword)
                        .accessibilityLabel("Save password")
                }

                Section {
                    Button {
                        session.connect()
                    } label: {
                        if session.isConnecting {
                            HStack {
                                ProgressView()
                                Text("Connecting…")
                            }
                        } else {
                            Text("Connect")
                        }
                    }
                    .disabled(session.isConnecting)
                    .accessibilityIdentifier("connectButton")
                }

                if case .error(let message) = session.status {
                    Section {
                        Text(message)
                            .foregroundStyle(.red)
                            .accessibilityIdentifier("errorMessage")
                    }
                } else if case .disconnected(let reason) = session.status {
                    Section {
                        Text(reason)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .navigationTitle("Gromnie")
        }
    }
}