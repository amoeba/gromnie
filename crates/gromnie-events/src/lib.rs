/// Core event system traits and types for gromnie
///
/// This crate provides the foundational types for the event system,
/// allowing different crates to implement consumers without circular dependencies.
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub mod client_events;
pub mod script_events;
pub mod simple_client_actions;
pub mod simple_game_events;
pub mod system_events;

// Re-export key types for convenience
pub use client_events::{ClientEvent, ClientStateEvent, ClientSystemEvent};
pub use script_events::ScriptEventType;
pub use simple_client_actions::SimpleClientAction;
pub use simple_game_events::{CharacterInfo, SimpleGameEvent};
pub use system_events::SystemEvent;

// ============================================================================
// Event Source and Context
// ============================================================================

/// Source of the event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    /// Event originated from network (server message)
    Network,
    /// Event originated from client internal state change
    ClientInternal,
    /// Event originated from a script
    Script,
    /// Event originated from system/lifecycle
    System,
}

/// Context information attached to all events
#[derive(Debug, Clone)]
pub struct EventContext {
    /// ID of the client that generated/processed this event
    pub client_id: u32,
    /// Sequence number for this event, relative to the client
    pub client_sequence: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl EventContext {
    pub fn new(client_id: u32, client_sequence: u64) -> Self {
        Self {
            client_id,
            client_sequence,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Unified event type (enriched event from the runner's perspective)
#[derive(Debug, Clone)]
pub enum EventType {
    Game(SimpleGameEvent),
    State(ClientStateEvent),
    System(SystemEvent),
}

// ============================================================================
// Event Envelope
// ============================================================================

/// Complete event envelope
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub event: EventType,
    pub context: EventContext,
    pub timestamp: Instant,
    pub source: EventSource,
}

impl EventEnvelope {
    pub fn new(event: EventType, context: EventContext, source: EventSource) -> Self {
        Self {
            event,
            context,
            timestamp: Instant::now(),
            source,
        }
    }

    pub fn game_event(
        game_event: SimpleGameEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(EventType::Game(game_event), context, source)
    }

    pub fn state_event(
        state_event: ClientStateEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(EventType::State(state_event), context, source)
    }

    pub fn system_event(
        system_event: SystemEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(EventType::System(system_event), context, source)
    }

    pub fn extract_game_event(&self) -> Option<SimpleGameEvent> {
        match &self.event {
            EventType::Game(game_event) => Some(game_event.clone()),
            _ => None,
        }
    }
}

// ============================================================================
// Event Consumer Trait
// ============================================================================

/// Trait for consuming game events - allows different implementations for CLI vs TUI
pub trait EventConsumer: Send + 'static {
    /// Handle an event envelope
    fn handle_event(&mut self, envelope: EventEnvelope);
}

// ============================================================================
// Consumer Factory System
// ============================================================================

/// Context provided to consumer factories when creating consumers
///
/// The Config type parameter is generic to avoid circular dependencies.
/// Most consumers use the default `()` type since they only need `client_id` and `action_tx`.
/// Advanced use cases can provide a custom config type if needed.
pub struct ConsumerContext<'a, Config = ()> {
    /// The client ID
    pub client_id: u32,
    /// The client configuration (type provided by consumer)
    ///
    /// Note: Most consumers use `Config = ()` to avoid circular dependencies.
    /// This field is available for advanced consumers that need configuration access.
    pub client_config: &'a Config,
    /// Channel to send actions back to the client
    pub action_tx: UnboundedSender<SimpleClientAction>,
}

/// Factory trait for creating event consumers
///
/// This trait allows consumers to be created lazily when the client is ready,
/// providing access to the client's action channel and configuration.
pub trait ConsumerFactory<Config = ()>: Send + Sync + 'static {
    /// Create a consumer for the given client context
    fn create(&self, ctx: &ConsumerContext<Config>) -> Box<dyn EventConsumer>;
}

// Allow closures to be used as consumer factories
impl<F, Config> ConsumerFactory<Config> for F
where
    F: Fn(&ConsumerContext<Config>) -> Box<dyn EventConsumer> + Send + Sync + 'static,
    Config: 'static,
{
    fn create(&self, ctx: &ConsumerContext<Config>) -> Box<dyn EventConsumer> {
        (self)(ctx)
    }
}
