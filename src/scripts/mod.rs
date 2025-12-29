/// Built-in scripts for Gromnie
///
/// This module contains example scripts that demonstrate the scripting API
/// and provide useful functionality out of the box.
pub mod auto_create_char;
pub mod auto_greet;
pub mod auto_login;
pub mod debug_logger;
pub mod hello_world;

use crate::scripting::ScriptRegistry;

/// Create a registry with all built-in scripts
pub fn create_registry() -> ScriptRegistry {
    let mut registry = ScriptRegistry::new();

    // Register all scripts
    registry.register("hello_world", || {
        Box::new(hello_world::HelloWorldScript::default())
    });
    registry.register("auto_greet", || {
        Box::new(auto_greet::AutoGreetScript)
    });
    registry.register("auto_login", || {
        Box::new(auto_login::AutoLoginScript::default())
    });
    registry.register("auto_create_char", || {
        Box::new(auto_create_char::AutoCreateCharScript::default())
    });
    registry.register("debug_logger", || {
        Box::new(debug_logger::DebugLoggerScript)
    });

    registry
}
