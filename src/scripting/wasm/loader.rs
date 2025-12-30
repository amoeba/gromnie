use std::path::{Path, PathBuf};
use tracing::{info, warn};
use wasmtime::Engine;

use super::WasmScript;
use crate::scripting::Script;

/// Load all WASM scripts from a directory
///
/// This function must run outside of an async runtime context because wasmtime-wasi
/// tries to set up its own runtime. We spawn a separate thread to avoid conflicts.
pub fn load_wasm_scripts(engine: &Engine, dir: &Path) -> Vec<WasmScript> {
    // Clone the engine for the thread (Engine is cheaply cloneable)
    let engine = engine.clone();
    let dir = dir.to_path_buf();

    // Spawn a thread to load WASM scripts outside the async runtime
    let handle = std::thread::spawn(move || load_wasm_scripts_blocking(&engine, &dir));

    // Wait for the thread to complete and return the result
    handle.join().unwrap_or_else(|_| {
        warn!(target: "scripting", "WASM script loading thread panicked");
        Vec::new()
    })
}

/// Internal blocking implementation of WASM script loading
fn load_wasm_scripts_blocking(engine: &Engine, dir: &Path) -> Vec<WasmScript> {
    let mut scripts = Vec::new();

    // Check if directory exists
    if !dir.exists() {
        info!(
            target: "scripting",
            "WASM directory does not exist: {} (this is fine if no WASM scripts are being used)",
            dir.display()
        );
        return scripts;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!(
                target: "scripting",
                "Failed to read WASM directory {}: {}",
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

        match WasmScript::from_file(engine, &path) {
            Ok(script) => {
                info!(
                    target: "scripting",
                    "Loaded WASM script: {} ({}) from {}",
                    script.name(),
                    script.id(),
                    path.display()
                );
                scripts.push(script);
            }
            Err(e) => {
                warn!(
                    target: "scripting",
                    "Failed to load WASM script {}: {:#}",
                    path.display(),
                    e
                );
            }
        }
    }

    if scripts.is_empty() {
        info!(
            target: "scripting",
            "No WASM scripts found in {}",
            dir.display()
        );
    } else {
        info!(
            target: "scripting",
            "Loaded {} WASM script(s)",
            scripts.len()
        );
    }

    scripts
}

/// Get the default WASM scripts directory
pub fn get_wasm_dir() -> PathBuf {
    use directories::ProjectDirs;
    let proj_dirs =
        ProjectDirs::from("", "", "gromnie").expect("Failed to determine config directory");
    proj_dirs.config_dir().join("wasm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_wasm_dir() {
        let dir = get_wasm_dir();
        assert!(dir.to_string_lossy().contains("gromnie"));
        assert!(dir.to_string_lossy().contains("wasm"));
    }
}
