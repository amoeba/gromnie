use std::sync::Mutex;
use std::time::Instant;

use crate::crypto::crypto_system::CryptoSystem;

/// Session state received from the server's ConnectRequest packet
/// This is now used internally within ClientSession
#[derive(Debug)]
pub struct ConnectionState {
    pub cookie: u64,
    pub client_id: u16,
    pub table: u16, // Table/iteration value from packet header
    pub send_generator: Mutex<CryptoSystem>, // Client->Server checksum encryption (initialized from seed_c2s)
}

impl Clone for ConnectionState {
    fn clone(&self) -> Self {
        Self {
            cookie: self.cookie,
            client_id: self.client_id,
            table: self.table,
            send_generator: Mutex::new(
                self.send_generator
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone(),
            ),
        }
    }
}

/// Account credentials
#[derive(Clone, Debug)]
pub struct Account {
    pub name: String,
    pub password: String,
}

// ========== New Session-Based Architecture ==========

/// Pure ACE protocol state (1:1 mapping with ACE server states)
#[derive(Clone, Debug, PartialEq)]
pub enum SessionState {
    /// Sent LoginRequest, waiting for ConnectRequest
    AuthLoginRequest,
    /// Received ConnectRequest, sent ConnectResponse
    AuthConnectResponse,
    /// Authenticated and connected (patching phase, character select, etc.)
    AuthConnected,
    /// Connected to world server
    WorldConnected,
    /// Termination started
    TerminationStarted,
}

/// Operational metadata (timing, retries, etc.)
#[derive(Clone, Debug)]
pub struct SessionMetadata {
    pub started_at: Option<Instant>,
    pub last_retry_at: Option<Instant>,
    pub connect_attempt_count: u32,
}

impl SessionMetadata {
    pub fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            last_retry_at: None,
            connect_attempt_count: 0,
        }
    }
}

/// Top-level session container (protocol state + metadata)
#[derive(Clone, Debug)]
pub struct ClientSession {
    pub state: SessionState,
    pub metadata: SessionMetadata,
    pub connection: Option<ConnectionState>,
}

impl ClientSession {
    pub fn new(state: SessionState) -> Self {
        Self {
            state,
            metadata: SessionMetadata::new(),
            connection: None,
        }
    }

    pub fn transition_to(&mut self, state: SessionState) {
        self.state = state;
    }

    pub fn set_connection(&mut self, connection: ConnectionState) {
        self.connection = Some(connection);
    }
}
