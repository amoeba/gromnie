use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::warn;
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
    let handle =
        std::thread::spawn(move || load_wasm_scripts_blocking(&engine, &dir, &script_config));

    // Wait for the thread to complete and return the result
    match handle.join() {
        Ok(scripts) => scripts,
        Err(panic_payload) => {
            // Try to extract panic message
            let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            tracing::error!(target: "scripting", "Script loading thread panicked: {}", panic_msg);
            Vec::new()
        }
    }
}

/// Internal blocking implementation of script loading
fn load_wasm_scripts_blocking(
    engine: &Engine,
    dir: &Path,
    script_config: &HashMap<String, toml::Value>,
) -> Vec<WasmScript> {
    use tracing::debug;

    let mut scripts = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Check if directory exists
    if !dir.exists() {
        debug!(
            target: "scripting",
            "Script directory does not exist: {}",
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
        let script = match WasmScript::from_file(engine, &path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    target: "scripting",
                    "Failed to load script {}: {:#}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        let script_id = script.id();

        // Skip if we've already loaded a script with this ID (handles duplicate .wasm files)
        if !seen_ids.insert(script_id.to_string()) {
            warn!(
                target: "scripting",
                "Duplicate script ID '{}' found in {}, skipping",
                script_id,
                path.display()
            );
            continue;
        }

        // Check if this script is enabled in config
        // Default to enabled if not specified in config
        let is_enabled = script_config
            .get(script_id)
            .and_then(|config: &toml::Value| config.get("enabled"))
            .and_then(|v: &toml::Value| v.as_bool())
            .unwrap_or(true); // Default to true

        if !is_enabled {
            debug!(
                target: "scripting",
                "Skipping disabled script: {} ({})",
                script.name(),
                script_id
            );
            continue;
        }

        debug!(
            target: "scripting",
            "Loaded: {} ({})",
            script.name(),
            script_id
        );
        scripts.push(script);
    }

    scripts
}

/// Get the default scripts directory
pub fn get_wasm_dir() -> PathBuf {
    use gromnie_client::config::ProjectPaths;
    let proj_paths = ProjectPaths::new("gromnie").expect("Failed to determine config directory");
    proj_paths.data_dir().join("scripts")
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
