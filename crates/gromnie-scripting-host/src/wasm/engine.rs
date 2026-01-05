use anyhow::{Context, Result};
use std::path::PathBuf;
use wasmtime::{Config, Engine};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};

use gromnie_client::config::ProjectPaths;

/// Create a configured Wasmtime engine for script execution
pub fn create_engine() -> Result<Engine> {
    let mut config = Config::new();

    // Enable component model support
    config.wasm_component_model(true);

    // Disable async support - scripts run synchronously
    config.async_support(false);

    // Enable optimizations for release builds
    #[cfg(not(debug_assertions))]
    {
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
    }

    Engine::new(&config).context("Failed to create Wasmtime engine")
}

/// Create WASI context with preopened script_data directory
pub fn create_wasi_context() -> Result<WasiCtx> {
    let script_data_dir = get_script_data_path()?;

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&script_data_dir).context("Failed to create script_data directory")?;

    Ok(WasiCtxBuilder::new()
        .inherit_stdio()
        .preopened_dir(
            &script_data_dir,
            "/script_data",
            DirPerms::all(),
            FilePerms::all(),
        )?
        .build())
}

/// Get the path to the script data directory
fn get_script_data_path() -> Result<PathBuf> {
    let proj_paths =
        ProjectPaths::new("gromnie").context("Failed to determine config directory")?;
    Ok(proj_paths.data_dir().join("script_data"))
}
