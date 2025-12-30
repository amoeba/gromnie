/// Core scripting system for Gromnie
///
/// This module provides a trait-based scripting system where scripts are Rust modules
/// compiled directly into the binary. Scripts can register interest in events and
/// reactively perform client actions through a well-defined lifecycle API.
pub mod context;
pub mod registry;
pub mod script;
pub mod script_runner;
pub mod timer;

// Re-export commonly used types
pub use context::{ClientStateSnapshot, ScriptContext};
pub use registry::{ScriptFactory, ScriptRegistry};
pub use script::{EventFilter, Script};
pub use script_runner::ScriptRunner;
pub use timer::{TimerId, TimerManager};
