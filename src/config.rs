use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub username: String,
    pub password: String,
}

impl std::fmt::Display for AccountConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.username)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub servers: BTreeMap<String, ServerConfig>,
    pub accounts: BTreeMap<String, AccountConfig>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
            path.push(".config/gromnie/config.toml");
            path
        }

        #[cfg(not(target_os = "macos"))]
        {
            use directories::ProjectDirs;
            let proj_dirs =
                ProjectDirs::from("", "", "gromnie").expect("Failed to determine config directory");
            proj_dirs.config_dir().join("config.toml")
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();

        if !path.exists() {
            return Err("Config file not found".into());
        }

        let content = fs::read_to_string(&path)?;
        let config = toml::from_str(&content)?;
        info!("Loaded config from {}", path.display());
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(&self)?;
        fs::write(&path, content)?;
        info!("Saved config to {}", path.display());
        Ok(())
    }
}
