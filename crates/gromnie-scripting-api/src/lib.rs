//! Script API for writing WASM scripts for Gromnie
//!
//! This crate provides the minimal API needed to write WASM scripts.
//! Scripts should depend on this crate, not on gromnie-scripting-host.

// Embedded WIT content for script binding generation
#[doc(hidden)]
pub const WIT_CONTENT: &str = include_str!("wit/gromnie-script.wit");

// Re-export wit_bindgen for the register_script! macro
#[doc(hidden)]
pub use wit_bindgen;

// Generate bindings from WIT at compile time for script use
// This generates unsafe functions that require unsafe blocks in Rust 2024
#[expect(unsafe_op_in_unsafe_fn)]
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
/// impl gromnie::Script for MyScript {
///     fn new() -> Self {
///         MyScript { /* ... */ }
///     }
///
///     fn id(&self) -> &str {
///         "my_script"
///     }
///
///     fn name(&self) -> &str {
///         "My Script"
///     }
///
///     fn description(&self) -> &str {
///         "Does something cool"
///     }
///
///     fn on_load(&mut self) {
///         gromnie::log("Script loaded!");
///     }
///
///     fn on_unload(&mut self) {}
///
///     fn subscribed_events(&self) -> Vec<u32> {
///         vec![]
///     }
///
///     fn on_event(&mut self, event: gromnie::ScriptEvent) {}
///
///     fn on_tick(&mut self, delta_millis: u64) {}
/// }
///
/// gromnie::register_script!(MyScript);
/// ```
pub trait WasmScript {
    /// Create a new instance of the script
    fn new() -> Self
    where
        Self: Sized;

    /// Unique identifier for this script
    fn id(&self) -> &str;

    /// Human-readable name for this script
    fn name(&self) -> &str;

    /// Description of what this script does
    fn description(&self) -> &str;

    /// Called when the script is first loaded
    fn on_load(&mut self);

    /// Called when the script is being unloaded
    fn on_unload(&mut self);

    /// Return the list of event IDs this script wants to receive
    fn subscribed_events(&self) -> Vec<u32>;

    /// Handle an event (game, state, or system)
    fn on_event(&mut self, event: ScriptEvent);

    /// Called periodically (delta_millis is time since last tick)
    fn on_tick(&mut self, delta_millis: u64);
}

pub use WasmScript as Script;

// Only compile this code when building for WASM targets or running tests
// This prevents linker errors when this crate is used as a dependency in non-WASM builds
#[cfg(any(target_family = "wasm", test))]
#[expect(unsafe_op_in_unsafe_fn)]
mod script_impl {
    use super::*;

    // Storage for script implementation (WASM is single-threaded)
    #[doc(hidden)]
    pub(super) static mut SCRIPT_IMPL: Option<Box<dyn WasmScript>> = None;

    // This function is defined by the register_script! macro
    // It will be defined in user code and linked into the WASM module
    unsafe extern "Rust" {
        fn __gromnie_script_constructor() -> Box<dyn WasmScript>;
    }

    #[doc(hidden)]
    pub(super) fn ensure_initialized() {
        #[expect(static_mut_refs)]
        unsafe {
            if SCRIPT_IMPL.is_none() {
                SCRIPT_IMPL = Some(__gromnie_script_constructor());
            }
        }
    }

    #[doc(hidden)]
    pub(super) fn script() -> &'static mut dyn WasmScript {
        ensure_initialized();

        #[expect(static_mut_refs)]
        unsafe {
            SCRIPT_IMPL
                .as_deref_mut()
                .expect("Script implementation missing")
        }
    }
}

#[cfg(any(target_family = "wasm", test))]
use script_impl::{ensure_initialized, script};

// Implement Guest trait to bridge to user's Script
// Only export this when building for WASM or tests
#[cfg(any(target_family = "wasm", test))]
export!(ScriptComponent);

#[cfg(any(target_family = "wasm", test))]
struct ScriptComponent;

#[cfg(any(target_family = "wasm", test))]
impl Guest for ScriptComponent {
    fn init() {
        ensure_initialized();
    }

    fn get_id() -> String {
        script().id().to_string()
    }

    fn get_name() -> String {
        script().name().to_string()
    }

    fn get_description() -> String {
        script().description().to_string()
    }

    fn on_load() {
        script().on_load()
    }

    fn on_unload() {
        script().on_unload()
    }

    fn subscribed_events() -> Vec<u32> {
        script().subscribed_events()
    }

    fn on_event(event: host::ScriptEvent) {
        script().on_event(event)
    }

    fn on_tick(delta_millis: u64) {
        script().on_tick(delta_millis)
    }
}

/// Register a script implementation
///
/// This macro stores your Script implementation and connects it to the WASM runtime.
///
/// # Usage
/// ```rust,ignore
/// use gromnie_scripting_api as gromnie;
///
/// struct MyScript {
///     // ... state
/// }
///
/// impl gromnie::Script for MyScript {
///     fn new() -> Self {
///         MyScript { /* ... */ }
///     }
///
///     fn id(&self) -> &str { "my_script" }
///     fn name(&self) -> &str { "My Script" }
///     // ... other methods ...
/// }
///
/// gromnie::register_script!(MyScript);
/// ```
#[macro_export]
macro_rules! register_script {
    ($script_type:ty) => {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub fn __gromnie_script_constructor() -> ::std::boxed::Box<dyn $crate::WasmScript> {
            ::std::boxed::Box::new(<$script_type as $crate::WasmScript>::new())
        }
    };
}
