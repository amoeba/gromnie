// ============================================================================

/// Configuration for running a client
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub id: u32,
    pub address: String,
    pub account_name: String,
    pub password: String,
    /// Optional character name to auto-login with after receiving character list
    pub character_name: Option<String>,
}

impl ClientConfig {
    /// Create a new client config
    pub fn new(id: u32, address: String, account_name: String, password: String) -> Self {
        Self {
            id,
            address,
            account_name,
            password,
            character_name: None,
        }
    }

    /// Set the character name for auto-login
    pub fn with_character_name(mut self, character_name: String) -> Self {
        self.character_name = Some(character_name);
        self
    }
}
