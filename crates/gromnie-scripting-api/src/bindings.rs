//! Re-exported WIT bindings for script authors
//!
//! Script writers should use the types from this module instead of
//! generating their own bindings. This module provides the generated
//! bindings from the WIT interface file.
//!
//! Note: This module re-exports the bindings generated in lib.rs.
//! Use the types from the crate root instead for better ergonomics.

// Re-export all the generated bindings from lib.rs
pub use crate::exports::gromnie::scripting::guest::Guest;
pub use crate::gromnie::scripting::host as host_interface;
pub use crate::gromnie::scripting::host;

// Re-export event types
pub use crate::gromnie::scripting::host::{GameEvent, ScriptEvent, StateEvent, SystemEvent};

// Re-export host functions for WASM scripts
pub use crate::gromnie::scripting::host::{
    cancel_timer, check_timer, get_client_state, get_event_time_millis, log, login_character,
    schedule_recurring, schedule_timer, send_chat,
};
