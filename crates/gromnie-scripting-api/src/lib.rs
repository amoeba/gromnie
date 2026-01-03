/// Script API for writing WASM scripts for Gromnie
///
/// This crate provides the minimal API needed to write WASM scripts.
/// Scripts should depend on this crate, not on gromnie-scripting-host.

// Embedded WIT content for script binding generation
#[doc(hidden)]
pub const WIT_CONTENT: &str = include_str!("wit/gromnie-script.wit");

// Re-export wit_bindgen for the register_script! macro
#[doc(hidden)]
pub use wit_bindgen;

// Generate bindings from WIT at compile time for script use
wit_bindgen::generate!({
    path: "src/wit",
    world: "script",
});

pub mod events;

// Re-export WASM script API at crate root for ergonomic imports
// This allows: use gromnie_scripting_api as gromnie; impl gromnie::WasmScript for MyScript
pub use exports::gromnie::scripting::guest::Guest;
pub use gromnie::scripting::host as host_interface;
pub use gromnie::scripting::host;

// Re-export event types
pub use gromnie::scripting::host::{GameEvent, ScriptEvent, StateEvent, SystemEvent};

// Re-export host functions for WASM scripts
pub use gromnie::scripting::host::{
    cancel_timer, check_timer, get_client_state, get_event_time_millis, log, login_character,
    schedule_recurring, schedule_timer, send_chat,
};

/// Trait for WASM script implementations
///
/// Implement this trait to create a script, then register it with
/// `register_script!(YourScript)`.
///
/// # Example
/// ```rust,ignore
/// use gromnie_scripting_api as gromnie;
///
/// struct MyScript {
///     // ... state
/// }
///
/// impl gromnie::WasmScript for MyScript {
///     // ... implement required methods
/// }
///
/// register_script!(MyScript);
/// ```
pub trait WasmScript {
    /// Called when the script is first loaded
    fn on_load(&mut self);

    /// Called when the script is being unloaded
    fn on_unload(&mut self);

    /// Return list of event filter IDs this script subscribes to
    ///
    /// Values correspond to EventFilter enum discriminants
    fn subscribed_events(&self) -> Vec<u32>;

    /// Handle a script event (game, state, or system)
    fn on_event(&mut self, event: ScriptEvent);

    /// Called periodically at fixed rate (default 20Hz)
    ///
    /// delta_millis: milliseconds since last tick
    fn on_tick(&mut self, delta_millis: u64);
}

pub use WasmScript as Script;
