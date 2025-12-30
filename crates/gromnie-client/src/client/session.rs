use std::cell::RefCell;

use crate::crypto::crypto_system::CryptoSystem;

/// Session state received from the server's ConnectRequest packet
#[derive(Clone, Debug)]
pub struct SessionState {
    pub cookie: u64,
    pub client_id: u16,
    pub table: u16, // Table/iteration value from packet header
    pub send_generator: RefCell<CryptoSystem>, // Client->Server checksum encryption (initialized from seed_c2s)
}

/// Account credentials
#[derive(Clone, Debug)]
pub struct Account {
    pub name: String,
    pub password: String,
}
