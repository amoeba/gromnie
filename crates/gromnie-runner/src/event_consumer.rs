use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::event_bus::{ClientStateEvent, EventEnvelope, EventType, ScriptEventType, SystemEvent};
use gromnie_client::client::events::{ClientAction, GameEvent};
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::time::Instant;

/// Trait for consuming game events - allows different implementations for CLI vs TUI
pub trait EventConsumer: Send + 'static {
    /// Handle an event envelope
    fn handle_event(&mut self, envelope: EventEnvelope);
}

/// Event consumer that logs events to the console (for CLI version)
pub struct LoggingConsumer {
    _action_tx: UnboundedSender<ClientAction>,
}

impl LoggingConsumer {
    pub fn new(action_tx: UnboundedSender<ClientAction>) -> Self {
        Self {
            _action_tx: action_tx,
        }
    }
}

impl EventConsumer for LoggingConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        // Extract GameEvent for backward compatibility
        let game_event = match envelope.event {
            EventType::Game(game_event) => game_event,
            EventType::State(state_event) => {
                match state_event {
                    ClientStateEvent::StateTransition { from, to, .. } => {
                        info!(target: "events", "STATE TRANSITION: {:?} -> {:?}", from, to);
                    }
                    ClientStateEvent::ClientFailed { reason, .. } => {
                        error!(target: "events", "CLIENT FAILED: {}", reason);
                    }
                }
                return;
            }
            EventType::System(system_event) => {
                match system_event {
                    SystemEvent::AuthenticationSucceeded { .. } => {
                        info!(target: "events", "Authentication succeeded - connected to server");
                    }
                    SystemEvent::AuthenticationFailed { reason, .. } => {
                        error!(target: "events", "Authentication failed: {}", reason);
                    }
                    SystemEvent::ConnectingStarted { .. } => {
                        info!(target: "events", "Connecting started");
                    }
                    SystemEvent::ConnectingDone { .. } => {
                        info!(target: "events", "Connecting done");
                    }
                    SystemEvent::UpdatingStarted { .. } => {
                        info!(target: "events", "Updating started");
                    }
                    SystemEvent::UpdatingDone { .. } => {
                        info!(target: "events", "Updating done");
                    }
                    SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
                    }
                    SystemEvent::ScriptEvent {
                        script_id,
                        event_type,
                    } => match event_type {
                        ScriptEventType::Loaded => {
                            info!(target: "events", "Script loaded: {}", script_id);
                        }
                        ScriptEventType::Unloaded => {
                            info!(target: "events", "Script unloaded: {}", script_id);
                        }
                        ScriptEventType::Error { message } => {
                            error!(target: "events", "Script error {}: {}", script_id, message);
                        }
                        ScriptEventType::Log { message } => {
                            info!(target: "events", "Script log {}: {}", script_id, message);
                        }
                    },
                    _ => {
                        // Handle other system events (e.g., Shutdown)
                    }
                }
                return;
            }
        };

        match game_event {
            GameEvent::CharacterListReceived {
                account,
                characters,
                num_slots,
            } => {
                let names = characters
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.id))
                    .collect::<Vec<_>>()
                    .join(", ");
                info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);
            }
            GameEvent::DDDInterrogation { language, region } => {
                info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
            }
            GameEvent::LoginFailed { reason } => {
                error!(target: "events", "LoginFailed -- Reason: {}", reason);
            }
            GameEvent::CreateObject {
                object_id,
                object_name,
            } => {
                info!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
            }
            GameEvent::ChatMessageReceived {
                message,
                message_type,
            } => {
                info!(target: "events", "CHAT [{}]: {}", message_type, message);
            }
            GameEvent::NetworkMessage {
                direction,
                message_type,
            } => {
                debug!(target: "events", "Network message: {:?} - {}", direction, message_type);
            }
            GameEvent::AuthenticationSucceeded => {
                info!(target: "events", "Authentication succeeded - connected to server");
            }
            GameEvent::AuthenticationFailed { reason } => {
                error!(target: "events", "Authentication failed: {}", reason);
            }
            // Ignore progress events in the CLI version
            GameEvent::ConnectingSetProgress { .. } => {}
            GameEvent::UpdatingSetProgress { .. } => {}
            GameEvent::ConnectingStart => {}
            GameEvent::ConnectingDone => {}
            GameEvent::UpdatingStart => {}
            GameEvent::UpdatingDone => {}
            GameEvent::CharacterError {
                error_code,
                error_message,
            } => {
                error!(target: "events", "Character error (code {}): {}", error_code, error_message);
            }
        }
    }
}

/// Event consumer that forwards events to TUI and logs to console
pub struct TuiConsumer {
    _action_tx: UnboundedSender<ClientAction>,
    tui_event_tx: UnboundedSender<GameEvent>,
}

impl TuiConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        tui_event_tx: UnboundedSender<GameEvent>,
    ) -> Self {
        tracing::info!(target: "tui_consumer", "Creating new TuiConsumer");
        Self {
            _action_tx: action_tx,
            tui_event_tx,
        }
    }
}

impl EventConsumer for TuiConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        tracing::info!(target: "tui_consumer", "TuiConsumer handling event: {:?}", std::mem::discriminant(&envelope.event));
        
        // ALWAYS forward game events to TUI - this is critical for game world updates
        if let Some(game_event) = envelope.extract_game_event() {
            let event_type = format!("{:?}", std::mem::discriminant(&game_event));
            match self.tui_event_tx.send(game_event.clone()) {
                Ok(_) => {
                    tracing::info!(target: "tui_events", "Forwarded to TUI: {}", event_type);
                }
                Err(e) => {
                    tracing::error!(target: "tui_events", "Failed to forward {} to TUI: {}", event_type, e);
                }
            }
        } else {
            tracing::debug!(target: "tui_consumer", "No GameEvent to forward from envelope");
        }

        // Log events for debugging (without early returns that would block forwarding)
        match &envelope.event {
            EventType::Game(game_event) => {
                // Log game events
                match game_event {
                    GameEvent::CharacterListReceived {
                        account,
                        characters,
                        num_slots,
                    } => {
                        let names = characters
                            .iter()
                            .map(|c| format!("{} ({})", c.name, c.id))
                            .collect::<Vec<_>>()
                            .join(", ");
                        info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);
                    }
                    GameEvent::DDDInterrogation { language, region } => {
                        info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
                    }
                    GameEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);
                    }
                    GameEvent::LoginFailed { reason } => {
                        error!(target: "events", "LoginFailed -- Reason {}", reason);
                    }
                    GameEvent::CreateObject {
                        object_id,
                        object_name,
                    } => {
                        info!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
                    }
                    GameEvent::ChatMessageReceived {
                        message,
                        message_type,
                    } => {
                        info!(target: "events", "CHAT [{}]: {}", message_type, message);
                    }
                    GameEvent::NetworkMessage {
                        direction,
                        message_type,
                    } => {
                        debug!(target: "events", "Network message: {:?} - {}", direction, message_type);
                    }
                    GameEvent::AuthenticationSucceeded => {
                        info!(target: "events", "Authentication succeeded - connected to server");
                    }
                    GameEvent::AuthenticationFailed { reason } => {
                        error!(target: "events", "Authentication failed: {}", reason);
                    }
                    GameEvent::ConnectingSetProgress { progress } => {
                        debug!(target: "events", "Connecting progress: {:.1}%", progress * 100.0);
                    }
                    GameEvent::UpdatingSetProgress { progress } => {
                        debug!(target: "events", "Updating progress: {:.1}%", progress * 100.0);
                    }
                    GameEvent::ConnectingStart => {
                        info!(target: "events", "Connecting started");
                    }
                    GameEvent::ConnectingDone => {
                        info!(target: "events", "Connecting done");
                    }
                    GameEvent::UpdatingStart => {
                        info!(target: "events", "Updating started");
                    }
                    GameEvent::UpdatingDone => {
                        info!(target: "events", "Updating done");
                    }
                    GameEvent::CharacterError {
                        error_code,
                        error_message,
                    } => {
                        error!(target: "events", "Character error (code {}): {}", error_code, error_message);
                    }
                }
            }
            EventType::State(state_event) => {
                match state_event {
                    crate::event_bus::ClientStateEvent::StateTransition { from, to, .. } => {
                        info!(target: "events", "STATE TRANSITION: {:?} -> {:?}", from, to);
                    }
                    crate::event_bus::ClientStateEvent::ClientFailed { reason, .. } => {
                        error!(target: "events", "CLIENT FAILED: {}", reason);
                    }
                }
            }
            EventType::System(system_event) => {
                match system_event {
                    crate::event_bus::SystemEvent::AuthenticationSucceeded { .. } => {
                        info!(target: "events", "Authentication succeeded - connected to server");
                    }
                    crate::event_bus::SystemEvent::AuthenticationFailed { reason, .. } => {
                        error!(target: "events", "Authentication failed: {}", reason);
                    }
                    crate::event_bus::SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);
                    }
                    crate::event_bus::SystemEvent::ConnectingStarted { .. } => {
                        info!(target: "events", "Connecting started");
                    }
                    crate::event_bus::SystemEvent::ConnectingDone { .. } => {
                        info!(target: "events", "Connecting done");
                    }
                    crate::event_bus::SystemEvent::UpdatingStarted { .. } => {
                        info!(target: "events", "Updating started");
                    }
                    crate::event_bus::SystemEvent::UpdatingDone { .. } => {
                        info!(target: "events", "Updating done");
                    }
                    _ => {
                        // Handle other system events if needed
                    }
                }
            }
        }
    }
}

/// Shared uptime data structure
#[derive(Clone)]
pub struct UptimeData {
    pub bot_start: Instant,
    pub ingame_start: Option<Instant>,
}

/// Event consumer that forwards chat messages to Discord
pub struct DiscordConsumer {
    _action_tx: UnboundedSender<ClientAction>,
    http: Arc<Http>,
    channel_id: ChannelId,
    bot_start_time: Instant,
    ingame_start_time: Option<Instant>,
    uptime_data: Option<Arc<tokio::sync::RwLock<UptimeData>>>,
}

impl DiscordConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        http: Arc<Http>,
        channel_id: ChannelId,
    ) -> Self {
        Self {
            _action_tx: action_tx,
            http,
            channel_id,
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: None,
        }
    }

    pub fn new_with_uptime(
        action_tx: UnboundedSender<ClientAction>,
        http: Arc<Http>,
        channel_id: ChannelId,
        uptime_data: Arc<tokio::sync::RwLock<UptimeData>>,
    ) -> Self {
        Self {
            _action_tx: action_tx,
            http,
            channel_id,
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: Some(uptime_data),
        }
    }
}

impl EventConsumer for DiscordConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        // Extract GameEvent for backward compatibility
        let game_event = match envelope.event {
            EventType::Game(game_event) => game_event,
            EventType::State(state_event) => {
                match state_event {
                    ClientStateEvent::StateTransition { from, to, .. } => {
                        info!(target: "events", "STATE TRANSITION: {:?} -> {:?}", from, to);
                    }
                    ClientStateEvent::ClientFailed { reason, .. } => {
                        error!(target: "events", "CLIENT FAILED: {}", reason);
                    }
                }
                return;
            }
            EventType::System(system_event) => {
                match system_event {
                    SystemEvent::AuthenticationSucceeded { .. } => {
                        info!(target: "events", "Authentication succeeded - connected to server");
                    }
                    SystemEvent::AuthenticationFailed { reason, .. } => {
                        error!(target: "events", "Authentication failed: {}", reason);
                    }
                    SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        // Record in-game start time
                        let now = Instant::now();
                        self.ingame_start_time = Some(now);

                        // Update shared uptime data if available
                        if let Some(ref uptime_data) = self.uptime_data {
                            let uptime_data_clone = uptime_data.clone();
                            tokio::spawn(async move {
                                let mut data = uptime_data_clone.write().await;
                                data.ingame_start = Some(now);
                            });
                        }

                        // Calculate total uptime
                        let total_uptime = self.bot_start_time.elapsed();
                        let total_secs = total_uptime.as_secs();
                        let total_hours = total_secs / 3600;
                        let total_mins = (total_secs % 3600) / 60;
                        let total_secs_remainder = total_secs % 60;

                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
                        info!(target: "events", "Bot uptime: {:02}:{:02}:{:02} | Now tracking in-game time", total_hours, total_mins, total_secs_remainder);
                        return;
                    }
                    _ => {
                        // Handle other system events if needed
                    }
                }
                return;
            }
        };

        match game_event {
            GameEvent::CharacterListReceived {
                account,
                characters,
                num_slots,
            } => {
                let names = characters
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.id))
                    .collect::<Vec<_>>()
                    .join(", ");
                info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);
            }
            GameEvent::ChatMessageReceived {
                message,
                message_type,
            } => {
                // Log with uptime info if available
                if let Some(ingame_start) = self.ingame_start_time {
                    let ingame_uptime = ingame_start.elapsed();
                    let ingame_secs = ingame_uptime.as_secs();
                    let ingame_hours = ingame_secs / 3600;
                    let ingame_mins = (ingame_secs % 3600) / 60;
                    let ingame_secs_remainder = ingame_secs % 60;
                    info!(target: "events", "CHAT [{}]: {} | In-game: {:02}:{:02}:{:02}", message_type, message, ingame_hours, ingame_mins, ingame_secs_remainder);
                } else {
                    info!(target: "events", "CHAT [{}]: {}", message_type, message);
                }

                // Forward to Discord
                let discord_message = format!("[{}] {}", message_type, message);
                let http = self.http.clone();
                let channel_id = self.channel_id;

                tokio::spawn(async move {
                    if let Err(e) = channel_id.say(&http, &discord_message).await {
                        error!("Failed to send Discord message: {}", e);
                    }
                });
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                // Record in-game start time
                let now = Instant::now();
                self.ingame_start_time = Some(now);

                // Update shared uptime data if available
                if let Some(ref uptime_data) = self.uptime_data {
                    let uptime_data_clone = uptime_data.clone();
                    tokio::spawn(async move {
                        let mut data = uptime_data_clone.write().await;
                        data.ingame_start = Some(now);
                    });
                }

                // Calculate total uptime
                let total_uptime = self.bot_start_time.elapsed();
                let total_secs = total_uptime.as_secs();
                let total_hours = total_secs / 3600;
                let total_mins = (total_secs % 3600) / 60;
                let total_secs_remainder = total_secs % 60;

                info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
                info!(target: "events", "Bot uptime: {:02}:{:02}:{:02} | Now tracking in-game time", total_hours, total_mins, total_secs_remainder);
            }
            GameEvent::LoginFailed { reason } => {
                error!(target: "events", "LoginFailed -- Reason: {}", reason);
            }
            GameEvent::DDDInterrogation { language, region } => {
                info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
            }
            GameEvent::CreateObject {
                object_id,
                object_name,
            } => {
                debug!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
            }
            GameEvent::NetworkMessage {
                direction,
                message_type,
            } => {
                debug!(target: "events", "Network message: {:?} - {}", direction, message_type);
            }
            GameEvent::AuthenticationSucceeded => {
                info!(target: "events", "Authentication succeeded - connected to server");
            }
            GameEvent::AuthenticationFailed { reason } => {
                error!(target: "events", "Authentication failed: {}", reason);
            }
            // Ignore progress events
            GameEvent::ConnectingSetProgress { .. } => {}
            GameEvent::UpdatingSetProgress { .. } => {}
            GameEvent::ConnectingStart => {}
            GameEvent::ConnectingDone => {}
            GameEvent::UpdatingStart => {}
            GameEvent::UpdatingDone => {}
            GameEvent::CharacterError {
                error_code,
                error_message,
            } => {
                error!(target: "events", "Character error (code {}): {}", error_code, error_message);
            }
        }
    }
}
