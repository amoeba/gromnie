// Refactored Event Bus Infrastructure
// Centralized event management where the runner owns the event bus

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, broadcast};

use gromnie_client::client::events::GameEvent;

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
    pub metadata: std::collections::HashMap<String, String>,
}

impl EventContext {
    pub fn new(client_id: u32, client_sequence: u64) -> Self {
        Self {
            client_id,
            client_sequence,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Client state transition events
#[derive(Debug, Clone)]
pub enum ClientStateEvent {
    StateTransition {
        from: String,
        to: String,
        client_id: u32,
    },
    ClientFailed {
        reason: String,
        client_id: u32,
    },
}

/// System and lifecycle events
#[derive(Debug, Clone)]
pub enum SystemEvent {
    AuthenticationSucceeded {
        client_id: u32,
    },
    AuthenticationFailed {
        client_id: u32,
        reason: String,
    },
    ConnectingStarted {
        client_id: u32,
    },
    ConnectingDone {
        client_id: u32,
    },
    UpdatingStarted {
        client_id: u32,
    },
    UpdatingDone {
        client_id: u32,
    },
    LoginSucceeded {
        character_id: u32,
        character_name: String,
    },
    ScriptEvent {
        script_id: String,
        event_type: ScriptEventType,
    },
    Shutdown {
        client_id: u32,
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

/// Unified event type (enriched event from the runner's perspective)
#[derive(Debug, Clone)]
pub enum EventType {
    Game(GameEvent),
    State(ClientStateEvent),
    System(SystemEvent),
}

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
        game_event: GameEvent,
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

    pub fn extract_game_event(&self) -> Option<GameEvent> {
        match &self.event {
            EventType::Game(game_event) => Some(game_event.clone()),
            _ => None,
        }
    }
}

/// Event sender that clients use to publish events
#[derive(Debug, Clone)]
pub struct EventSender {
    sender: broadcast::Sender<EventEnvelope>,
    client_id: u32,
}

impl EventSender {
    pub fn new(sender: broadcast::Sender<EventEnvelope>, client_id: u32) -> Self {
        Self { sender, client_id }
    }

    pub fn publish(&self, envelope: EventEnvelope) {
        let _ = self.sender.send(envelope);
    }

    pub fn client_id(&self) -> u32 {
        self.client_id
    }
}

/// Central event bus that manages event distribution
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<EventEnvelope>,
    #[allow(dead_code)]
    global_sequence_counter: Arc<Mutex<u64>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<EventEnvelope>) {
        let (sender, receiver) = broadcast::channel(capacity);
        let bus = EventBus {
            sender: sender.clone(),
            global_sequence_counter: Arc::new(Mutex::new(0)),
        };
        (bus, receiver)
    }

    pub fn create_sender(&self, client_id: u32) -> EventSender {
        EventSender::new(self.sender.clone(), client_id)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.sender.subscribe()
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Trait for components that can handle events
pub trait EventHandler: Send + 'static {
    fn handle_event(&mut self, envelope: EventEnvelope);
}

/// Simple event handler that logs all events
pub struct LoggingEventHandler;

impl EventHandler for LoggingEventHandler {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            EventType::Game(game_event) => {
                tracing::debug!(target: "events", "Game Event: {:?}", game_event);
            }
            EventType::State(state_event) => {
                tracing::info!(target: "events", "State Event: {:?}", state_event);
            }
            EventType::System(system_event) => {
                tracing::info!(target: "events", "System Event: {:?}", system_event);
            }
        }
    }
}
