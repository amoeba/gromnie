/// Host runtime for loading and executing WASM scripts
///
/// This crate provides the runtime for the game client to load and run WASM scripts.
/// Scripts should depend on gromnie-scripting-api, not this crate.
use gromnie_scripting_api as api;
use std::any::Any;
use std::time::Duration;

pub mod context;
pub mod registry;
pub mod reload;
pub mod script_runner;
pub mod timer;
pub mod wasm;

// Re-export commonly used types for host-side scripting
pub use api::Script as ApiScript;
pub use context::{ClientStateSnapshot, ScriptContext};
pub use reload::setup_reload_signal_handler;
pub use script_runner::{ScriptConsumer, ScriptRunner, create_script_consumer};
pub use timer::{TimerId, TimerManager};

// Registry is now just a utility function
pub use registry::create_runner_from_config;

// Host-friendly trait wrapper
/// Trait that scripts must implement (combines API and host requirements)
pub trait Script: Send + 'static {
    /// Unique identifier for this script (e.g., "hello_world")
    fn id(&self) -> &'static str;

    /// Human-readable name for this script
    fn name(&self) -> &'static str;

    /// Description of what this script does
    fn description(&self) -> &'static str;

    /// Called when the script is first loaded
    fn on_load(&mut self, ctx: &mut ScriptContext);

    /// Called when the script is being unloaded
    fn on_unload(&mut self, ctx: &mut ScriptContext);

    /// Return the list of events this script wants to receive
    fn subscribed_events(&self) -> &[EventFilter];

    /// Handle an event that matches one of the subscribed filters
    fn on_event(&mut self, event: &gromnie_events::SimpleGameEvent, ctx: &mut ScriptContext);

    /// Called periodically at a fixed rate (configurable, default ~20Hz)
    /// Use this for timer checks, periodic updates, and time-based logic
    ///
    /// # Arguments
    /// * `ctx` - Script context for accessing client state and sending actions
    /// * `delta` - Time elapsed since last tick
    fn on_tick(&mut self, ctx: &mut ScriptContext, delta: Duration);

    /// Allow downcasting to concrete script type for state access
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Filter for subscribing to specific game events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventFilter {
    /// Subscribe to all events
    All,
    /// Character list received from server
    CharacterListReceived,
    /// Object created in game world
    CreateObject,
    /// Chat message received
    ChatMessageReceived,
}

impl EventFilter {
    /// Check if this filter matches the given event
    pub fn matches(&self, event: &gromnie_events::SimpleGameEvent) -> bool {
        use gromnie_events::SimpleGameEvent as GameEvent;

        match self {
            EventFilter::All => true,
            EventFilter::CharacterListReceived => {
                matches!(event, GameEvent::CharacterListReceived { .. })
            }
            EventFilter::CreateObject => {
                matches!(event, GameEvent::CreateObject { .. })
            }
            EventFilter::ChatMessageReceived => {
                matches!(event, GameEvent::ChatMessageReceived { .. })
            }
        }
    }

    /// Convert a u32 discriminant to an EventFilter
    /// These discriminants correspond to the WIT interface event IDs
    pub fn from_discriminant(id: u32) -> Option<Self> {
        match id {
            0 => Some(EventFilter::All),
            1 => Some(EventFilter::CharacterListReceived),
            2 => Some(EventFilter::CreateObject),
            3 => Some(EventFilter::ChatMessageReceived),
            _ => None,
        }
    }

    /// Get the discriminant value for this event filter
    pub fn to_discriminant(&self) -> u32 {
        match self {
            EventFilter::All => 0,
            EventFilter::CharacterListReceived => 1,
            EventFilter::CreateObject => 2,
            EventFilter::ChatMessageReceived => 3,
        }
    }
}
