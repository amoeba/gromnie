use serde::{Deserialize, Serialize};

/// Configuration for client reconnection with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection when the connection is lost
    pub enabled: bool,

    /// Maximum number of reconnection attempts (0 = unlimited)
    pub max_attempts: u32,

    /// Initial retry delay in seconds (default: 10)
    pub initial_delay_secs: u64,

    /// Maximum retry delay in seconds (default: 600 = 10 minutes)
    pub max_delay_secs: u64,

    /// Exponential backoff multiplier - each failure multiplies delay by this amount (default: 2)
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_attempts: 0, // 0 means unlimited retries
            initial_delay_secs: 10,
            max_delay_secs: 600,
            backoff_multiplier: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Calculate the delay for a given retry attempt (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay_secs = (self.initial_delay_secs as f64
            * self.backoff_multiplier.powi(attempt as i32))
        .min(self.max_delay_secs as f64) as u64;
        std::time::Duration::from_secs(delay_secs)
    }

    /// Check if we should attempt reconnection based on max attempts
    pub fn should_attempt_reconnect(&self, attempt: u32) -> bool {
        if self.max_attempts == 0 {
            true // Unlimited retries
        } else {
            attempt < self.max_attempts
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}
