/// Core event system traits and types for gromnie
///
/// This crate provides the foundational types for the event system,
/// allowing different crates to implement consumers without circular dependencies.
use std::collections::HashMap;

pub mod client_events;
mod instant;
pub mod protocol_events;
pub mod script_events;
pub mod system_events;

use instant::Instant;

// Re-export key types for convenience
pub use client_events::{ClientEvent, ClientStateEvent, ClientSystemEvent};
pub use protocol_events::{
    ChatMessage, GameEventMsg, IntoGameEventMsg, OrderedGameEvent, ProtocolEvent, S2CEvent,
};
pub use script_events::ScriptEventType;
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
    Protocol(ProtocolEvent),
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

    pub fn protocol_event(
        protocol_event: ProtocolEvent,
        client_id: u32,
        client_sequence: u64,
        source: EventSource,
    ) -> Self {
        let context = EventContext::new(client_id, client_sequence);
        Self::new(EventType::Protocol(protocol_event), context, source)
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

    pub fn extract_protocol_event(&self) -> Option<ProtocolEvent> {
        match &self.event {
            EventType::Protocol(protocol_event) => Some(protocol_event.clone()),
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
