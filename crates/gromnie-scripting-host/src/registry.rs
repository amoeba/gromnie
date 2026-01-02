use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::script_runner::ScriptRunner;
use gromnie_client::{client::events::ClientAction, config::scripting_config::ScriptingConfig};

/// Create a script runner from config
pub fn create_runner_from_config(
    action_tx: UnboundedSender<ClientAction>,
    config: &ScriptingConfig,
) -> ScriptRunner {
    // Create runner with script support
    debug!(target: "scripting", "Creating script runner");
    let mut runner = ScriptRunner::new_with_wasm(action_tx);

    // Load scripts if enabled
    if config.enabled {
        let script_dir = config.script_dir();
        debug!(target: "scripting", "Loading scripts from: {}", script_dir.display());
        runner.load_scripts(&script_dir, &config.config);
    }

    runner
}
