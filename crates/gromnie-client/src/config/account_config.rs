use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub username: String,
    pub password: String,
    /// Optional character name to auto-login with after receiving character list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
}

impl std::fmt::Display for AccountConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.username)
    }
}
