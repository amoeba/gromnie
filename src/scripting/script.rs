use std::any::Any;
use std::time::Duration;

use super::context::ScriptContext;
use crate::client::events::GameEvent;

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
    /// DDD interrogation message
    DDDInterrogation,
    /// Character login succeeded
    LoginSucceeded,
    /// Character login failed
    LoginFailed,
    /// Object created in game world
    CreateObject,
    /// Chat message received
    ChatMessageReceived,
    /// Network message sent/received
    NetworkMessage,
    /// Connecting progress update
    ConnectingSetProgress,
    /// Updating progress update
    UpdatingSetProgress,
    /// Connecting phase started
    ConnectingStart,
    /// Connecting phase done
    ConnectingDone,
    /// Authentication succeeded
    AuthenticationSucceeded,
    /// Authentication failed
    AuthenticationFailed,
    /// Updating phase started
    UpdatingStart,
    /// Updating phase done
    UpdatingDone,
}

impl EventFilter {
    /// Check if this filter matches the given event
    pub fn matches(&self, event: &GameEvent) -> bool {
        match self {
            EventFilter::All => true,
            EventFilter::CharacterListReceived => {
                matches!(event, GameEvent::CharacterListReceived { .. })
            }
            EventFilter::DDDInterrogation => {
                matches!(event, GameEvent::DDDInterrogation { .. })
            }
            EventFilter::LoginSucceeded => {
                matches!(event, GameEvent::LoginSucceeded { .. })
            }
            EventFilter::LoginFailed => {
                matches!(event, GameEvent::LoginFailed { .. })
            }
            EventFilter::CreateObject => {
                matches!(event, GameEvent::CreateObject { .. })
            }
            EventFilter::ChatMessageReceived => {
                matches!(event, GameEvent::ChatMessageReceived { .. })
            }
            EventFilter::NetworkMessage => {
                matches!(event, GameEvent::NetworkMessage { .. })
            }
            EventFilter::ConnectingSetProgress => {
                matches!(event, GameEvent::ConnectingSetProgress { .. })
            }
            EventFilter::UpdatingSetProgress => {
                matches!(event, GameEvent::UpdatingSetProgress { .. })
            }
            EventFilter::ConnectingStart => {
                matches!(event, GameEvent::ConnectingStart)
            }
            EventFilter::ConnectingDone => {
                matches!(event, GameEvent::ConnectingDone)
            }
            EventFilter::AuthenticationSucceeded => {
                matches!(event, GameEvent::AuthenticationSucceeded)
            }
            EventFilter::AuthenticationFailed => {
                matches!(event, GameEvent::AuthenticationFailed { .. })
            }
            EventFilter::UpdatingStart => {
                matches!(event, GameEvent::UpdatingStart)
            }
            EventFilter::UpdatingDone => {
                matches!(event, GameEvent::UpdatingDone)
            }
        }
    }
}
