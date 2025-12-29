use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use super::script::Script;
use super::script_runner::ScriptRunner;
use crate::client::events::ClientAction;

/// Factory function type for creating script instances
pub type ScriptFactory = fn() -> Box<dyn Script>;

/// Registry of available scripts
pub struct ScriptRegistry {
    /// Map of script ID to factory function
    factories: HashMap<String, ScriptFactory>,
}

impl ScriptRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a script factory
    pub fn register(&mut self, id: impl Into<String>, factory: ScriptFactory) {
        let id = id.into();
        debug!(target: "scripting", "Registering script factory: {}", id);
        self.factories.insert(id, factory);
    }

    /// Create a script runner with the specified enabled scripts
    pub fn create_runner(
        &self,
        action_tx: UnboundedSender<ClientAction>,
        enabled_scripts: &[String],
    ) -> ScriptRunner {
        let mut runner = ScriptRunner::new(action_tx);

        for script_id in enabled_scripts {
            match self.factories.get(script_id) {
                Some(factory) => {
                    let script = factory();
                    debug!(target: "scripting", "Creating script instance: {}", script_id);
                    runner.register_script(script);
                }
                None => {
                    warn!(target: "scripting", "Unknown script ID in config: {}", script_id);
                }
            }
        }

        runner
    }

    /// Get the list of all registered script IDs
    pub fn available_scripts(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to register multiple scripts at once
///
/// # Example
/// ```ignore
/// let mut registry = ScriptRegistry::new();
/// register_scripts!(registry, HelloWorldScript, AutoGreetScript);
/// ```
#[macro_export]
macro_rules! register_scripts {
    ($registry:expr, $($script:ty),+ $(,)?) => {
        $(
            $registry.register(
                <$script as $crate::scripting::script::Script>::id(&<$script>::default()),
                || Box::new(<$script>::default()),
            );
        )+
    };
}
