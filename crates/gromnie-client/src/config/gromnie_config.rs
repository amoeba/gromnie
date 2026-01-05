use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::config::{
    account_config::AccountConfig, paths::ProjectPaths, scripting_config::ScriptingConfig,
    server_config::ServerConfig,
};

#[derive(Debug)]
pub enum ConfigLoadError {
    NotFound,
    ParseError(String),
    IoError(String),
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::NotFound => write!(f, "Config file not found"),
            ConfigLoadError::ParseError(msg) => write!(f, "Failed to parse config: {}", msg),
            ConfigLoadError::IoError(msg) => write!(f, "IO error reading config: {}", msg),
        }
    }
}

impl std::error::Error for ConfigLoadError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GromnieConfig {
    #[serde(default)]
    pub servers: BTreeMap<String, ServerConfig>,
    #[serde(default)]
    pub accounts: BTreeMap<String, AccountConfig>,

    /// Scripting configuration
    #[serde(default)]
    pub scripting: ScriptingConfig,

    /// Enable automatic reconnection with exponential backoff
    #[serde(default)]
    pub reconnect: bool,
}

impl GromnieConfig {
    pub fn config_path() -> PathBuf {
        let proj_paths =
            ProjectPaths::new("gromnie").expect("Failed to determine config directory");
        proj_paths.config_dir().join("config.toml")
    }

    pub fn load() -> Result<Self, ConfigLoadError> {
        let path = Self::config_path();

        if !path.exists() {
            return Err(ConfigLoadError::NotFound);
        }

        let content =
            fs::read_to_string(&path).map_err(|e| ConfigLoadError::IoError(e.to_string()))?;
        let config =
            toml::from_str(&content).map_err(|e| ConfigLoadError::ParseError(e.to_string()))?;
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
