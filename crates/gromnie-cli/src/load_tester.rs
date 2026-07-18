//! Client naming utilities for load testing
//!
//! Re-exported from gromnie-runner so the naming logic has a single source of truth.

pub use gromnie_runner::{ClientNaming, decode_client_id, encode_client_id};

/// Generate and print client naming information
pub fn generate_naming_info(client_id: u32) {
    let naming = ClientNaming::new(client_id);

    println!("Client ID: {}", client_id);
    println!("Account: {}", naming.account_name());
    println!("Password: {}", naming.password());
    println!("Character: {}", naming.character_name());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_values_from_shared_module() {
        let naming = ClientNaming::new(26);
        assert_eq!(naming.code(), "AABA");
        assert_eq!(naming.account_name(), "Load-AABA");
        assert_eq!(decode_client_id(&encode_client_id(26)), Some(26));
    }
}
