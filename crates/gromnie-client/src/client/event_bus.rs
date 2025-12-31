// Event bus infrastructure for the Gromnie client system
// This provides centralized event management across all components

use std::time::Instant;

use crate::client::events::{ClientAction, GameEvent};
use crate::client::ClientState;

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
    /// Sequence number for this event, relative to the client (resets per client)
    /// This provides ordering guarantees for events from the same client
    pub client_sequence: u64,
    /// Additional metadata (can be extended as needed)
    pub metadata: std::collections::HashMap<String, String>,
}

impl EventContext {
    /// Create a new event context
    ///
    /// # Parameters
    /// - `client_id`: The ID of the client generating this event
    /// - `client_sequence`: Sequence number relative to this client (not global)
    pub fn new(client_id: u32, client_sequence: u64) -> Self {
        Self {
            client_id,
            client_sequence,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Client state transition events
#[derive(Debug, Clone)]
pub enum ClientStateEvent {
    /// Client state transition occurred
    StateTransition {
        from: ClientState,
        to: ClientState,
        client_id: u32,
    },
    /// Client entered failed state
    ClientFailed {
        reason: String,
        client_id: u32,
    },
}

/// System and lifecycle events
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Authentication with server succeeded
    AuthenticationSucceeded {
        client_id: u32,
    },
    /// Authentication with server failed
    AuthenticationFailed {
        client_id: u32,
        reason: String,
    },
    /// Client started connecting
    ConnectingStarted {
        client_id: u32,
    },
    /// Client finished connecting
    ConnectingDone {
        client_id: u32,
    },
    /// Client started updating/patching
    UpdatingStarted {
        client_id: u32,
    },
    /// Client finished updating/patching
    UpdatingDone {
        client_id: u32,
    },
    /// Scripting system event
    ScriptEvent {
        script_id: String,
        event_type: ScriptEventType,
    },
}

/// Types of script-related events
#[derive(Debug, Clone)]
pub enum ScriptEventType {
    Loaded,
    Unloaded,
    Error { message: String },
    Log { message: String },
}

/// Unified event enum that categorizes all event types
#[derive(Debug, Clone)]
pub enum ClientEvent {
    /// Game events from the server
    Game(GameEvent),
    /// Client state transition events
    State(ClientStateEvent),
    /// System and lifecycle events
    System(SystemEvent),
}

/// Complete event envelope with context and metadata
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    /// The actual event payload
    pub event: ClientEvent,
    /// Context information about this event
    pub context: EventContext,
    /// When this event was created
    pub timestamp: Instant,
    /// Where this event originated from
    pub source: EventSource,
}

impl EventEnvelope {
    /// Create a new event envelope
    pub fn new(
        event: ClientEvent,
        context: EventContext,
        source: EventSource,
    ) -> Self {
        Self {
            event,
            context,
            timestamp: Instant::now(),
            source,
        }
    }

    /// Create a game event envelope
    ///
    /// # Parameters
    /// - `game_event`: The game event to wrap
    /// - `client_id`: ID of the client generating this event
    /// - `client_sequence`: Sequence number relative to this client
    /// - `source`: Where this event originated from
    pub fn game_event(
        game_event: GameEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(ClientEvent::Game(game_event), context, source)
    }

    /// Create a state event envelope
    ///
    /// # Parameters
    /// - `state_event`: The state event to wrap
    /// - `client_id`: ID of the client generating this event
    /// - `client_sequence`: Sequence number relative to this client
    /// - `source`: Where this event originated from
    pub fn state_event(
        state_event: ClientStateEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(ClientEvent::State(state_event), context, source)
    }

    /// Create a system event envelope
    ///
    /// # Parameters
    /// - `system_event`: The system event to wrap
    /// - `client_id`: ID of the client generating this event
    /// - `client_sequence`: Sequence number relative to this client
    /// - `source`: Where this event originated from
    pub fn system_event(
        system_event: SystemEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(ClientEvent::System(system_event), context, source)
    }

    /// Extract GameEvent if this is a game event (for backward compatibility)
    pub fn extract_game_event(&self) -> Option<GameEvent> {
        match &self.event {
            ClientEvent::Game(game_event) => Some(game_event.clone()),
            _ => None,
        }
    }
}

/// Central event bus that manages event distribution
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<EventEnvelope>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(capacity: usize) -> (Self, tokio::sync::broadcast::Receiver<EventEnvelope>) {
        let (sender, receiver) = tokio::sync::broadcast::channel(capacity);
        (EventBus { sender }, receiver)
    }

    /// Publish an event to the bus
    pub fn publish(&self, envelope: EventEnvelope) {
        // Ignore errors if there are no subscribers
        let _ = self.sender.send(envelope);
    }

    /// Get the number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Create a new subscriber
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<EventEnvelope> {
        self.sender.subscribe()
    }
}

/// Trait for components that can handle events
pub trait EventHandler: Send + 'static {
    /// Handle an event envelope
    fn handle_event(&mut self, envelope: EventEnvelope);
}

/// Simple event handler that logs all events (for debugging)
pub struct LoggingEventHandler;

impl EventHandler for LoggingEventHandler {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            ClientEvent::Game(game_event) => {
                tracing::debug!(target: "events", "Game Event: {:?}", game_event);
            }
            ClientEvent::State(state_event) => {
                tracing::info!(target: "events", "State Event: {:?}", state_event);
            }
            ClientEvent::System(system_event) => {
                tracing::info!(target: "events", "System Event: {:?}", system_event);
            }
        }
    }
}