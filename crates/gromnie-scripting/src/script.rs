use std::any::Any;
use std::time::Duration;

use super::context::ScriptContext;
use gromnie_client::client::events::GameEvent;

/// Trait that all scripts must implement
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
    fn on_event(&mut self, event: &GameEvent, ctx: &mut ScriptContext);

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
    pub fn matches(&self, event: &GameEvent) -> bool {
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
}
