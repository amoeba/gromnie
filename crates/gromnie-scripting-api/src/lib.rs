/// Script API for writing WASM scripts for Gromnie
///
/// This crate provides the minimal API needed to write WASM scripts.
/// Scripts should depend on this crate, not on gromnie-scripting-host.

// Embedded WIT content for script binding generation
#[doc(hidden)]
pub const WIT_CONTENT: &str = include_str!("wit/gromnie-script.wit");

#[cfg(feature = "wasm")]
// Re-export wit_bindgen for the register_script! macro
#[doc(hidden)]
pub use wit_bindgen;

#[cfg(feature = "wasm")]
pub mod bindings;
pub mod events;

#[cfg(feature = "wasm")]
// Re-export WASM script API at crate root for ergonomic imports
// This allows: use gromnie_scripting_api as gromnie; impl gromnie::WasmScript for MyScript
pub use bindings::WasmScript;
#[cfg(feature = "wasm")]
pub use bindings::WasmScript as Script;
#[cfg(feature = "wasm")]
pub use bindings::host::GameEvent;

// Re-export host functions for WASM scripts
#[cfg(feature = "wasm")]
pub use bindings::host::{
    cancel_timer, check_timer, get_client_state, get_event_time_millis, log, login_character,
    schedule_recurring, schedule_timer, send_chat,
};
