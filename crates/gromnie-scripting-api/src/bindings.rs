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

// Re-export the main types that scripts will use
pub use self::exports::gromnie::scripting::guest::Guest;
pub use self::gromnie::scripting::host as host_interface;

// Expose the raw generated module structure for direct imports
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
pub static mut SCRIPT_IMPL: Option<Box<dyn WasmScript>> = None;

#[doc(hidden)]
pub fn register_script_impl(build_script: fn() -> Box<dyn WasmScript>) {
    unsafe {
        SCRIPT_IMPL = Some((build_script)());
    }
}

#[doc(hidden)]
pub static mut SCRIPT_INIT_FN: Option<fn() -> Box<dyn WasmScript>> = None;

/// Initialize the script if not already initialized
#[doc(hidden)]
pub fn init_script() {
    #[expect(static_mut_refs)]
    unsafe {
        // Initialize lazily if needed - SCRIPT_INIT_FN is set by register_script! at compile time
        if SCRIPT_IMPL.is_none()
            && let Some(init_fn) = SCRIPT_INIT_FN
        {
            SCRIPT_IMPL = Some((init_fn)());
        }
    }
}

fn script() -> &'static mut dyn WasmScript {
    #[expect(static_mut_refs)]
    unsafe {
        // Initialize lazily if needed - SCRIPT_INIT_FN is set by register_script! at compile time
        if SCRIPT_IMPL.is_none() {
            if let Some(init_fn) = SCRIPT_INIT_FN {
                SCRIPT_IMPL = Some((init_fn)());
            } else {
                panic!("Script not initialized. Did you call register_script! macro?");
            }
        }
        SCRIPT_IMPL
            .as_deref_mut()
            .expect("Script implementation is missing")
    }
}

// Implement Guest trait to bridge to user's Script
export!(ScriptComponent);

struct ScriptComponent;

impl Guest for ScriptComponent {
    fn init() {
        // Initialize script on first call
        // Call the macro-generated init function if it exists
        unsafe extern "C" {
            #[link_name = "init-script"]
            fn __call_init_script();
        }

        unsafe {
            __call_init_script();
        }
    }

    fn get_id() -> String {
        // Initialize script if needed (lazy initialization)
        init_script();
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
        // Store the constructor function in a module-level function
        #[doc(hidden)]
        pub fn __gromnie_script_constructor() -> ::std::boxed::Box<dyn $crate::bindings::WasmScript>
        {
            ::std::boxed::Box::new(<$script_type as $crate::bindings::WasmScript>::new())
        }

        // Export a function the host can call to set up initialization
        #[doc(hidden)]
        #[unsafe(export_name = "init-script")]
        pub extern "C" fn __init_script() {
            #[expect(unsafe_op_in_unsafe_fn)]
            unsafe {
                // Set up the init function on first call
                if $crate::bindings::SCRIPT_INIT_FN.is_none() {
                    $crate::bindings::SCRIPT_INIT_FN = Some(__gromnie_script_constructor);
                }
                // Perform initialization
                $crate::bindings::init_script();
            }
        }
    };
}
