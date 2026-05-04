use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::script_runner::ScriptRunner;
use gromnie_client::client::Client;
use gromnie_client::config::scripting_config::ScriptingConfig;
use gromnie_events::SimpleClientAction;

/// Create a script runner from config
///
/// Note: This only creates the runner with WASM support enabled.
/// Scripts are loaded separately via `with_reload_config()` to avoid duplicate loading.
pub fn create_runner_from_config(
    client: Arc<RwLock<Client>>,
    action_tx: UnboundedSender<SimpleClientAction>,
    config: &ScriptingConfig,
) -> ScriptRunner {
    // Create runner with script support and configured timeout
    debug!(target: "scripting", "Creating script runner with {}ms timeout", config.script_timeout_ms);
    let timeout = std::time::Duration::from_millis(config.script_timeout_ms);
    ScriptRunner::new_with_wasm_and_config(client, action_tx, timeout)
}
