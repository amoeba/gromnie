use std::sync::Arc;
use tokio::sync::mpsc;

use crate::event_bus::{EventContext, EventEnvelope, EventSource, EventType, SystemEvent};
use gromnie_events::{ClientEvent, ClientSystemEvent};

/// Wraps raw events from client and enriches them with context
pub struct EventWrapper {
    client_id: u32,
    sequence_counter: u64,
    event_bus: Arc<crate::event_bus::EventBus>,
}

impl EventWrapper {
    pub fn new(client_id: u32, event_bus: Arc<crate::event_bus::EventBus>) -> Self {
        Self {
            client_id,
            sequence_counter: 0,
            event_bus,
        }
    }

    /// Run the event wrapper task
    /// Receives raw events and publishes enriched envelopes to the event bus
    pub async fn run(mut self, mut raw_rx: mpsc::Receiver<ClientEvent>) {
        let sender = self.event_bus.create_sender(self.client_id);
        tracing::info!(target: "event_wrapper", "EventWrapper started for client {}", self.client_id);
        while let Some(raw_event) = raw_rx.recv().await {
            tracing::debug!(target: "event_wrapper", "EventWrapper received ClientEvent: {:?}", std::mem::discriminant(&raw_event));
            let envelope = self.wrap_event(raw_event);
            tracing::debug!(target: "event_wrapper", "EventWrapper publishing envelope: {:?}", std::mem::discriminant(&envelope.event));
            sender.publish(envelope);
            self.sequence_counter += 1;
        }
        tracing::warn!(target: "event_wrapper", "EventWrapper stopped - no more events");
    }

    fn wrap_event(&mut self, raw: ClientEvent) -> EventEnvelope {
        let source = self.determine_source(&raw);
        let context = EventContext::new(self.client_id, self.sequence_counter);

        let event = match raw {
            ClientEvent::Game(game) => EventType::Game(game),
            ClientEvent::State(state) => EventType::State(state),
            ClientEvent::System(sys) => EventType::System(self.convert_system_event(sys)),
            ClientEvent::Protocol(_) => {
                // Protocol events are not yet fully supported in the event bus
                // For now, convert to a placeholder system event
                tracing::debug!(target: "event_wrapper", "Protocol event received, converting to System event");
                EventType::System(SystemEvent::Disconnected {
                    client_id: self.client_id,
                    will_reconnect: false,
                    reconnect_attempt: 0,
                    delay_secs: 0,
                })
            }
        };

        EventEnvelope::new(event, context, source)
    }

    fn determine_source(&self, raw: &ClientEvent) -> EventSource {
        match raw {
            ClientEvent::Game(_) => EventSource::Network,
            ClientEvent::State(_) => EventSource::ClientInternal,
            ClientEvent::System(_) => EventSource::System,
            ClientEvent::Protocol(_) => EventSource::Network,
        }
    }

    fn convert_system_event(&self, sys: ClientSystemEvent) -> SystemEvent {
        match sys {
            ClientSystemEvent::AuthenticationSucceeded => SystemEvent::AuthenticationSucceeded {
                client_id: self.client_id,
            },
            ClientSystemEvent::AuthenticationFailed { reason } => {
                SystemEvent::AuthenticationFailed {
                    client_id: self.client_id,
                    reason,
                }
            }
            ClientSystemEvent::ConnectingStarted => SystemEvent::ConnectingStarted {
                client_id: self.client_id,
            },
            ClientSystemEvent::ConnectingDone => SystemEvent::ConnectingDone {
                client_id: self.client_id,
            },
            ClientSystemEvent::UpdatingStarted => SystemEvent::UpdatingStarted {
                client_id: self.client_id,
            },
            ClientSystemEvent::UpdatingDone => SystemEvent::UpdatingDone {
                client_id: self.client_id,
            },
            ClientSystemEvent::LoginSucceeded {
                character_id,
                character_name,
            } => SystemEvent::LoginSucceeded {
                character_id,
                character_name,
            },
            ClientSystemEvent::Disconnected {
                will_reconnect,
                reconnect_attempt,
                delay_secs,
            } => SystemEvent::Disconnected {
                client_id: self.client_id,
                will_reconnect,
                reconnect_attempt,
                delay_secs,
            },
            ClientSystemEvent::Reconnecting {
                attempt,
                delay_secs,
            } => SystemEvent::Reconnecting {
                client_id: self.client_id,
                attempt,
                delay_secs,
            },
        }
    }
}
