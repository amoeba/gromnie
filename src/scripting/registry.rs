use std::collections::{HashMap, HashSet};
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
        let mut loaded = HashSet::new();

        for script_id in enabled_scripts {
            // Skip if already loaded
            if !loaded.insert(script_id.clone()) {
                warn!(target: "scripting", "Script '{}' listed multiple times in config, skipping duplicate", script_id);
                continue;
            }

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

    /// Create a script runner from config (supports both Rust and WASM scripts)
    pub fn create_runner_from_config(
        &self,
        action_tx: UnboundedSender<ClientAction>,
        config: &crate::config::ScriptingConfig,
    ) -> ScriptRunner {
        // Create runner with WASM support if enabled
        let mut runner = if config.wasm_enabled {
            debug!(target: "scripting", "Creating script runner with WASM support");
            ScriptRunner::new_with_wasm(action_tx)
        } else {
            ScriptRunner::new(action_tx)
        };

        // Load Rust scripts from registry
        let mut loaded = HashSet::new();
        for script_id in &config.enabled_scripts {
            // Skip if already loaded
            if !loaded.insert(script_id.clone()) {
                warn!(target: "scripting", "Script '{}' listed multiple times in config, skipping duplicate", script_id);
                continue;
            }

            match self.factories.get(script_id) {
                Some(factory) => {
                    let script = factory();
                    debug!(target: "scripting", "Creating Rust script instance: {}", script_id);
                    runner.register_script(script);
                }
                None => {
                    warn!(target: "scripting", "Unknown Rust script ID in config: {}", script_id);
                }
            }
        }

        // Load WASM scripts if enabled
        if config.wasm_enabled {
            let wasm_dir = config.wasm_dir();
            debug!(target: "scripting", "Loading WASM scripts from: {}", wasm_dir.display());
            runner.load_wasm_scripts(&wasm_dir);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::events::GameEvent;
    use crate::scripting::{EventFilter, ScriptContext};
    use std::any::Any;
    use std::time::Duration;

    /// Test script that counts how many times it's been created
    #[derive(Default)]
    struct TestScript {
        #[allow(dead_code)]
        instance_id: usize,
    }

    // Use a static counter to track instances across test script creations
    static mut INSTANCE_COUNTER: usize = 0;

    impl TestScript {
        fn new() -> Self {
            unsafe {
                INSTANCE_COUNTER += 1;
                Self {
                    instance_id: INSTANCE_COUNTER,
                }
            }
        }

        fn reset_counter() {
            unsafe {
                INSTANCE_COUNTER = 0;
            }
        }
    }

    impl Script for TestScript {
        fn id(&self) -> &'static str {
            "test_script"
        }

        fn name(&self) -> &'static str {
            "Test Script"
        }

        fn description(&self) -> &'static str {
            "A test script for unit testing"
        }

        fn on_load(&mut self, _ctx: &mut ScriptContext) {}

        fn on_unload(&mut self, _ctx: &mut ScriptContext) {}

        fn subscribed_events(&self) -> &[EventFilter] {
            &[]
        }

        fn on_event(&mut self, _event: &GameEvent, _ctx: &mut ScriptContext) {}

        fn on_tick(&mut self, _ctx: &mut ScriptContext, _delta: Duration) {}

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_nonexistent_script_in_config() {
        // Setup
        let mut registry = ScriptRegistry::new();
        registry.register("test_script", || Box::new(TestScript::new()));

        // Create runner with a mix of valid and invalid script IDs
        let (action_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let enabled_scripts = vec![
            "test_script".to_string(),
            "nonexistent_script".to_string(),
            "another_missing_script".to_string(),
        ];

        let runner = registry.create_runner(action_tx, &enabled_scripts);

        // Assert: Only the valid script should be loaded
        assert_eq!(
            runner.script_count(),
            1,
            "Should only load the 1 valid script"
        );
        assert_eq!(
            runner.script_ids(),
            vec!["test_script"],
            "Should only have test_script loaded"
        );
    }

    #[test]
    fn test_duplicate_script_in_config() {
        // Reset the instance counter before the test
        TestScript::reset_counter();

        // Setup
        let mut registry = ScriptRegistry::new();
        registry.register("test_script", || Box::new(TestScript::new()));

        // Create runner with duplicate script IDs
        let (action_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let enabled_scripts = vec![
            "test_script".to_string(),
            "test_script".to_string(),
            "test_script".to_string(),
        ];

        let runner = registry.create_runner(action_tx, &enabled_scripts);

        // Assert: Only one instance should be created (duplicates are filtered)
        assert_eq!(
            runner.script_count(),
            1,
            "Duplicate scripts should be filtered, only loading once"
        );
        assert_eq!(
            runner.script_ids(),
            vec!["test_script"],
            "Only one instance should be registered despite duplicates in config"
        );
    }

    #[test]
    fn test_available_scripts() {
        // Setup
        let mut registry = ScriptRegistry::new();
        registry.register("script_one", || Box::new(TestScript::new()));
        registry.register("script_two", || Box::new(TestScript::new()));
        registry.register("script_three", || Box::new(TestScript::new()));

        // Get available scripts
        let mut available = registry.available_scripts();
        available.sort(); // HashMap order is not guaranteed

        // Assert
        assert_eq!(available.len(), 3);
        assert!(available.contains(&"script_one".to_string()));
        assert!(available.contains(&"script_two".to_string()));
        assert!(available.contains(&"script_three".to_string()));
    }

    #[test]
    fn test_empty_enabled_scripts() {
        // Setup
        let mut registry = ScriptRegistry::new();
        registry.register("test_script", || Box::new(TestScript::new()));

        // Create runner with empty enabled scripts
        let (action_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let enabled_scripts: Vec<String> = vec![];

        let runner = registry.create_runner(action_tx, &enabled_scripts);

        // Assert: No scripts should be loaded
        assert_eq!(runner.script_count(), 0, "Should not load any scripts");
    }
}
