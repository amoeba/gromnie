// Refactored Event Bus Infrastructure
// Centralized event management where the runner owns the event bus

use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

// Re-export types from gromnie-events for convenience
pub use gromnie_events::{
    ClientStateEvent, EventContext, EventEnvelope, EventSource, EventType, ScriptEventType,
    SystemEvent,
};

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

/// Unified event type specifically for the TUI
/// This allows the TUI to receive all types of events
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Game(gromnie_events::SimpleGameEvent),
    System(SystemEvent),
    State(ClientStateEvent),
}

impl From<gromnie_events::SimpleGameEvent> for TuiEvent {
    fn from(event: gromnie_events::SimpleGameEvent) -> Self {
        TuiEvent::Game(event)
    }
}

impl From<SystemEvent> for TuiEvent {
    fn from(event: SystemEvent) -> Self {
        TuiEvent::System(event)
    }
}

impl From<ClientStateEvent> for TuiEvent {
    fn from(event: ClientStateEvent) -> Self {
        TuiEvent::State(event)
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
            EventType::Input(keyboard_event) => {
                tracing::debug!(target: "events", "Input Event: {:?}", keyboard_event);
            }
        }
    }
}
