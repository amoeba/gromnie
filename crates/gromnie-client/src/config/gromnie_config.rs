use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::config::{
    account_config::AccountConfig, scripting_config::ScriptingConfig, server_config::ServerConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GromnieConfig {
    pub servers: BTreeMap<String, ServerConfig>,
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
        use directories::ProjectDirs;
        let proj_dirs =
            ProjectDirs::from("", "", "gromnie").expect("Failed to determine config directory");
        proj_dirs.config_dir().join("config.toml")
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
