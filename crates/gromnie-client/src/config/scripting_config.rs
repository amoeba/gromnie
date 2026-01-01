use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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
}

impl Default for ScriptingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            script_dir: None,
            config: HashMap::new(),
        }
    }
}

impl ScriptingConfig {
    /// Get the script directory path (use provided or default)
    pub fn script_dir(&self) -> PathBuf {
        self.script_dir.clone().unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "gromnie")
                .map(|d| d.data_dir().join("scripts"))
                .unwrap_or_else(|| PathBuf::from(".scripts"))
        })
    }
}
