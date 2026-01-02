//! Client naming utilities for load testing
//!
//! Converts client IDs to unique account and character names using base-26 encoding.
//! Supports up to 456,976 clients (26^4).

/// Convert a client ID to a 4-character base-26 code (AAAA to ZZZZ)
pub fn encode_client_id(id: u32) -> String {
    // Clamp to max valid ID (26^4 - 1 = 456975)
    let id = id.min(456975);

    let mut id = id;
    let mut code = String::with_capacity(4);

    for _ in 0..4 {
        let char_idx = (id % 26) as u8;
        code.insert(0, (b'A' + char_idx) as char);
        id /= 26;
    }

    code
}

/// Decode a 4-character base-26 code back to a client ID
///
/// Returns None if the string is not exactly 4 characters or contains non-A-Z characters.
#[allow(dead_code)]
pub fn decode_client_id(code: &str) -> Option<u32> {
    if code.len() != 4 {
        return None;
    }

    let mut id: u32 = 0;

    for (pos, ch) in code.chars().enumerate() {
        if !ch.is_ascii_uppercase() {
            return None;
        }

        let digit = ch as u32 - b'A' as u32;
        let power = 4 - 1 - pos; // positions from left: 3, 2, 1, 0
        id += digit * 26_u32.pow(power as u32);
    }

    Some(id)
}

/// Encapsulates client naming for load testing
pub struct ClientNaming {
    #[allow(dead_code)]
    client_id: u32,
    code: String,
}

impl ClientNaming {
    /// Create naming for a client with the given ID
    pub fn new(client_id: u32) -> Self {
        let code = encode_client_id(client_id);
        Self { client_id, code }
    }

    /// Get the account name for this client
    ///
    /// Format: Load-XXXX (e.g., Load-AAAA)
    pub fn account_name(&self) -> String {
        format!("Load-{}", self.code)
    }

    /// Get the password for this client (same as account name)
    pub fn password(&self) -> String {
        self.account_name()
    }

    /// Get the character name for this client
    ///
    /// Format: Load-XXXX-A (e.g., Load-AAAA-A)
    pub fn character_name(&self) -> String {
        format!("{}-A", self.account_name())
    }

    /// Get the 4-character code for this client
    #[allow(dead_code)]
    pub fn code(&self) -> &str {
        &self.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        for id in [0, 1, 25, 26, 27, 100, 675, 456975] {
            let encoded = encode_client_id(id);
            let decoded = decode_client_id(&encoded).expect("decode should succeed");
            assert_eq!(
                id, decoded,
                "Roundtrip failed for id {}: {} -> {} -> {}",
                id, encoded, encoded, decoded
            );
        }
    }

    #[test]
    fn test_encode_known_values() {
        assert_eq!(encode_client_id(0), "AAAA");
        assert_eq!(encode_client_id(1), "AAAB");
        assert_eq!(encode_client_id(25), "AAAZ");
        assert_eq!(encode_client_id(26), "AABA");
        assert_eq!(encode_client_id(27), "AABB");
    }

    #[test]
    fn test_client_naming() {
        let naming = ClientNaming::new(0);
        assert_eq!(naming.code(), "AAAA");
        assert_eq!(naming.account_name(), "Load-AAAA");
        assert_eq!(naming.password(), "Load-AAAA");
        assert_eq!(naming.character_name(), "Load-AAAA-A");
    }

    #[test]
    fn test_max_clients() {
        // Verify we can handle the max number of clients
        let max_id = 456975; // 26^4 - 1
        let naming = ClientNaming::new(max_id);
        assert_eq!(naming.code(), "ZZZZ");
        assert_eq!(naming.account_name(), "Load-ZZZZ");

        // Verify clamp behavior for values beyond max
        let over_max = ClientNaming::new(456976);
        assert_eq!(over_max.code(), "ZZZZ");
    }
}
