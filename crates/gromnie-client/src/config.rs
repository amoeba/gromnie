use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
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

/// Helper function for default true value
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptingConfig {
    /// Whether scripting is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// List of Rust script IDs to enable
    #[serde(default)]
    pub enabled_scripts: Vec<String>,

    /// Enable WASM scripting
    #[serde(default)]
    pub wasm_enabled: bool,

    /// Directory containing WASM scripts (default: ~/.config/gromnie/wasm)
    #[serde(default)]
    pub wasm_dir: Option<PathBuf>,

    /// Per-script configuration (script ID -> config values)
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enabled_scripts: Vec::new(),
            wasm_enabled: false,
            wasm_dir: None,
            config: HashMap::new(),
        }
    }
}

impl ScriptingConfig {
    /// Get the WASM directory path (use provided or default)
    pub fn wasm_dir(&self) -> PathBuf {
        self.wasm_dir.clone().unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "gromnie")
                .map(|d| d.data_dir().join("wasm"))
                .unwrap_or_else(|| PathBuf::from(".wasm"))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub servers: BTreeMap<String, ServerConfig>,
    pub accounts: BTreeMap<String, AccountConfig>,

    /// Scripting configuration
    #[serde(default)]
    pub scripting: ScriptingConfig,
}

impl Config {
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
