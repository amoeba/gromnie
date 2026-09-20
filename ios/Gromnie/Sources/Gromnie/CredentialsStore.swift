import Foundation
import Security

/// Persists connection preferences. Host/port/username live in `UserDefaults`;
/// the password is only written to the Keychain when the user opts in.
final class CredentialsStore {
    private let defaults = UserDefaults.standard
    private let service = "net.gromnie.ios"

    private enum Key {
        static let host = "connection.host"
        static let port = "connection.port"
        static let username = "connection.username"
        static let savePassword = "connection.savePassword"
    }

    var host: String { defaults.string(forKey: Key.host) ?? "" }
    var port: String { defaults.string(forKey: Key.port) ?? "9000" }
    var username: String { defaults.string(forKey: Key.username) ?? "" }
    var savePassword: Bool { defaults.bool(forKey: Key.savePassword) }

    func loadPassword(host: String, port: String, username: String) -> String {
        guard !host.isEmpty, !username.isEmpty else { return "" }
        return readKeychain(account: account(host: host, port: port, username: username)) ?? ""
    }

    func save(host: String, port: String, username: String, password: String?, savePassword: Bool) {
        defaults.set(host, forKey: Key.host)
        defaults.set(port, forKey: Key.port)
        defaults.set(username, forKey: Key.username)
        defaults.set(savePassword, forKey: Key.savePassword)

        let account = account(host: host, port: port, username: username)
        if savePassword, let password, !password.isEmpty {
            writeKeychain(account: account, password: password)
        } else {
            deleteKeychain(account: account)
        }
    }

    private func account(host: String, port: String, username: String) -> String {
        "\(host):\(port):\(username)"
    }

    // MARK: - Keychain

    private func readKeychain(account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private func writeKeychain(account: String, password: String) {
        deleteKeychain(account: account)
        let attributes: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
            kSecValueData as String: Data(password.utf8),
        ]
        SecItemAdd(attributes as CFDictionary, nil)
    }

    private func deleteKeychain(account: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        SecItemDelete(query as CFDictionary)
    }
}