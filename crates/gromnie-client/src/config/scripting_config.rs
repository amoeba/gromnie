use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::paths::ProjectPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptingConfig {
    /// Whether scripting is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Directory containing scripts (default: ~/.config/gromnie/scripts)
    #[serde(default)]
    pub script_dir: Option<PathBuf>,

    /// Per-script configuration (script ID -> config values)
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,

    /// Whether hot reload is enabled (default: true)
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,

    /// Hot reload scan interval in milliseconds (default: 500ms)
    #[serde(default = "default_hot_reload_interval")]
    pub hot_reload_interval_ms: u64,
}

fn default_hot_reload() -> bool {
    true
}

fn default_hot_reload_interval() -> u64 {
    1000
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            script_dir: None,
            config: HashMap::new(),
            hot_reload: true,
            hot_reload_interval_ms: 1000,
        }
    }
}

impl ScriptingConfig {
    /// Get the script directory path (use provided or default)
    pub fn script_dir(&self) -> PathBuf {
        self.script_dir.clone().unwrap_or_else(|| {
            ProjectPaths::new("gromnie")
                .map(|p| p.data_dir().join("scripts"))
                .unwrap_or_else(|| PathBuf::from(".scripts"))
        })
    }
}
