use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use wasmtime::Engine;

use super::WasmScript;
use crate::Script;

/// Load all scripts from a directory, filtering by config
///
/// This function must run outside of an async runtime context because wasmtime-wasi
/// tries to set up its own runtime. We spawn a separate thread to avoid conflicts.
pub fn load_wasm_scripts(
    engine: &Engine,
    dir: &Path,
    script_config: &HashMap<String, toml::Value>,
) -> Vec<WasmScript> {
    // Clone the engine for the thread (Engine is cheaply cloneable)
    let engine = engine.clone();
    let dir = dir.to_path_buf();
    let script_config = script_config.clone();

    // Spawn a thread to load scripts outside the async runtime
    let handle = std::thread::spawn(move || {
        load_wasm_scripts_blocking(&engine, &dir, &script_config)
    });

    // Wait for the thread to complete and return the result
    handle.join().unwrap_or_else(|_| {
        warn!(target: "scripting", "Script loading thread panicked");
        Vec::new()
    })
}

/// Internal blocking implementation of script loading
fn load_wasm_scripts_blocking(
    engine: &Engine,
    dir: &Path,
    script_config: &HashMap<String, toml::Value>,
) -> Vec<WasmScript> {
    let mut scripts = Vec::new();

    // Check if directory exists
    if !dir.exists() {
        info!(
            target: "scripting",
            "Script directory does not exist: {} (this is fine if no scripts are being used)",
            dir.display()
        );
        return scripts;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(
                target: "scripting",
                "Failed to read script directory {}: {}",
                dir.display(),
                e
            );
            return scripts;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only load .wasm files
        if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            continue;
        }

        // Try to load the script to get its ID
        let script_result = WasmScript::from_file(engine, &path);

        let script = match script_result {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    target: "scripting",
                    "Failed to load script {}: {:#}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        let script_id = script.id();

        // Check if this script is enabled in config
        // Default to enabled if not specified in config
        let is_enabled = script_config
            .get(script_id)
            .and_then(|config: &toml::Value| config.get("enabled"))
            .and_then(|v: &toml::Value| v.as_bool())
            .unwrap_or(true); // Default to true

        if !is_enabled {
            info!(
                target: "scripting",
                "Skipping disabled script: {} ({}) from {}",
                script.name(),
                script_id,
                path.display()
            );
            continue;
        }

        info!(
            target: "scripting",
            "Loaded script: {} ({}) from {}",
            script.name(),
            script_id,
            path.display()
        );
        scripts.push(script);
    }

    if scripts.is_empty() {
        info!(
            target: "scripting",
            "No scripts found in {}",
            dir.display()
        );
    } else {
        info!(
            target: "scripting",
            "Loaded {} script(s)",
            scripts.len()
        );
    }

    scripts
}

/// Get the default scripts directory
pub fn get_wasm_dir() -> PathBuf {
    use directories::ProjectDirs;
    let proj_dirs =
        ProjectDirs::from("", "", "gromnie").expect("Failed to determine config directory");
    proj_dirs.config_dir().join("scripts")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_wasm_dir() {
        let dir = get_wasm_dir();
        assert!(dir.to_string_lossy().contains("gromnie"));
        assert!(dir.to_string_lossy().contains("scripts"));
    }
}
