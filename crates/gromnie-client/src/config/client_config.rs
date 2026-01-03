// ============================================================================

use crate::config::server_config::ReconnectConfig;

/// Configuration for running a client
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub id: u32,
    pub address: String,
    pub account_name: String,
    pub password: String,
    pub reconnect: ReconnectConfig,
}

impl ClientConfig {
    /// Create a new client config
    pub fn new(id: u32, address: String, account_name: String, password: String) -> Self {
        Self {
            id,
            address,
            account_name,
            password,
            reconnect: ReconnectConfig::default(),
        }
    }

    /// Set the reconnection configuration
    pub fn with_reconnect(mut self, reconnect: ReconnectConfig) -> Self {
        self.reconnect = reconnect;
        self
    }
}
