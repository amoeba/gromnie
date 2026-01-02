use gromnie_client::client::events::ClientAction;
use gromnie_client::client::events::GameEvent;
/// Core event system traits and types for gromnie
///
/// This crate provides the foundational types for the event system,
/// allowing different crates to implement consumers without circular dependencies.
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

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

// ============================================================================
// Event Types
// ============================================================================

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

/// Types of script-related events
#[derive(Debug, Clone)]
pub enum ScriptEventType {
    Loaded,
    Unloaded,
    Error { message: String },
    Log { message: String },
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
    /// Request to reload all scripts (triggered by SIGUSR2 or other mechanism)
    ReloadScripts,
    Shutdown {
        client_id: u32,
    },
}

/// Unified event type (enriched event from the runner's perspective)
#[derive(Debug, Clone)]
pub enum EventType {
    Game(GameEvent),
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

// ============================================================================
// Event Consumer Trait
// ============================================================================

/// Trait for consuming game events - allows different implementations for CLI vs TUI
pub trait EventConsumer: Send + 'static {
    /// Handle an event envelope
    fn handle_event(&mut self, envelope: EventEnvelope);
}

// ============================================================================
// Client Configuration
// ============================================================================

/// Configuration for running a client
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub id: u32,
    pub address: String,
    pub account_name: String,
    pub password: String,
}

impl ClientConfig {
    /// Create a new client config
    pub fn new(id: u32, address: String, account_name: String, password: String) -> Self {
        Self {
            id,
            address,
            account_name,
            password,
        }
    }
}

// ============================================================================
// Consumer Factory System
// ============================================================================

/// Context provided to consumer factories when creating consumers
pub struct ConsumerContext<'a> {
    /// The client ID
    pub client_id: u32,
    /// The client configuration
    pub client_config: &'a ClientConfig,
    /// Channel to send actions back to the client
    pub action_tx: UnboundedSender<ClientAction>,
}

/// Factory trait for creating event consumers
///
/// This trait allows consumers to be created lazily when the client is ready,
/// providing access to the client's action channel and configuration.
pub trait ConsumerFactory: Send + Sync + 'static {
    /// Create a consumer for the given client context
    fn create(&self, ctx: &ConsumerContext) -> Box<dyn EventConsumer>;
}

// Allow closures to be used as consumer factories
impl<F> ConsumerFactory for F
where
    F: Fn(&ConsumerContext) -> Box<dyn EventConsumer> + Send + Sync + 'static,
{
    fn create(&self, ctx: &ConsumerContext) -> Box<dyn EventConsumer> {
        (self)(ctx)
    }
}
