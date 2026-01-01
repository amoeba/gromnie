//! Re-exported WIT bindings for script authors
//!
//! Script writers should use the types from this module instead of
//! generating their own bindings. This module provides the generated
//! bindings from the WIT interface file.

#![allow(unsafe_op_in_unsafe_fn)]

// Generate bindings from WIT at compile time for script use
wit_bindgen::generate!({
    path: "src/wit",
    world: "script",
});

pub use self::exports::gromnie::scripting::guest::Guest;
pub use self::gromnie::scripting::host as host_interface;
pub use self::gromnie::scripting::host;

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
///     fn on_event(&mut self, event: gromnie::GameEvent) {}
///
///     fn on_tick(&mut self, delta_millis: u64) {}
/// }
///
/// gromnie::register_script!(MyScript);
/// ```
pub trait WasmScript: Send + 'static {
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

    /// Handle an event
    fn on_event(&mut self, event: host::GameEvent);

    /// Called periodically (delta_millis is time since last tick)
    fn on_tick(&mut self, delta_millis: u64);
}

// Storage for script implementation (WASM is single-threaded)
#[doc(hidden)]
static mut SCRIPT_IMPL: Option<Box<dyn WasmScript>> = None;

// This function is defined by the register_script! macro
// It will be defined in user code and linked into the WASM module
unsafe extern "Rust" {
    fn __gromnie_script_constructor() -> Box<dyn WasmScript>;
}

#[doc(hidden)]
fn ensure_initialized() {
    #[expect(static_mut_refs)]
    unsafe {
        if SCRIPT_IMPL.is_none() {
            SCRIPT_IMPL = Some(__gromnie_script_constructor());
        }
    }
}

#[doc(hidden)]
fn script() -> &'static mut dyn WasmScript {
    ensure_initialized();

    #[expect(static_mut_refs)]
    unsafe {
        SCRIPT_IMPL
            .as_deref_mut()
            .expect("Script implementation missing")
    }
}

// Implement Guest trait to bridge to user's Script
export!(ScriptComponent);

struct ScriptComponent;

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

    fn on_event(event: host::GameEvent) {
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
        #[no_mangle]
        fn __gromnie_script_constructor() -> ::std::boxed::Box<dyn $crate::bindings::WasmScript> {
            ::std::boxed::Box::new(<$script_type as $crate::bindings::WasmScript>::new())
        }
    };
}
