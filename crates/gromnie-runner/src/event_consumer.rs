use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use gromnie_client::client::events::{ClientAction, GameEvent};
use gromnie_client::config::ScriptingConfig;
use gromnie_scripting::ScriptRunner;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::time::Instant;

/// Trait for consuming game events - allows different implementations for CLI vs TUI
pub trait EventConsumer: Send + 'static {
    /// Handle a game event
    fn handle_event(&mut self, event: GameEvent);
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
    fn handle_event(&mut self, event: GameEvent) {
        match event {
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
            GameEvent::ConnectingSetProgress { .. }
            | GameEvent::UpdatingSetProgress { .. }
            | GameEvent::ConnectingStart
            | GameEvent::ConnectingDone
            | GameEvent::UpdatingStart
            | GameEvent::UpdatingDone => {}
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
        Self {
            _action_tx: action_tx,
            tui_event_tx,
        }
    }
}

impl EventConsumer for TuiConsumer {
    fn handle_event(&mut self, event: GameEvent) {
        // Forward all events to TUI
        let _ = self.tui_event_tx.send(event.clone());

        // Handle specific events with logging
        match event {
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
                info!(target: "events", "LoginSucceeded --  Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);
            }
            GameEvent::LoginFailed { reason } => {
                error!(target: "events", "LoginFailed -- Reason {}", reason);
            }
            GameEvent::CreateObject {
                object_id,
                object_name,
            } => {
                debug!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
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
            // Progress events are handled by TUI directly
            GameEvent::ConnectingSetProgress { .. }
            | GameEvent::UpdatingSetProgress { .. }
            | GameEvent::ConnectingStart
            | GameEvent::ConnectingDone
            | GameEvent::UpdatingStart
            | GameEvent::UpdatingDone => {}
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
    fn handle_event(&mut self, event: GameEvent) {
        match event {
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
            GameEvent::ConnectingSetProgress { .. }
            | GameEvent::UpdatingSetProgress { .. }
            | GameEvent::ConnectingStart
            | GameEvent::ConnectingDone
            | GameEvent::UpdatingStart
            | GameEvent::UpdatingDone => {}
        }
    }
}

/// Wrapper around ScriptRunner that implements EventConsumer trait
pub struct ScriptConsumer {
    runner: ScriptRunner,
}

impl ScriptConsumer {
    pub fn new(runner: ScriptRunner) -> Self {
        Self { runner }
    }
}

impl EventConsumer for ScriptConsumer {
    fn handle_event(&mut self, event: GameEvent) {
        self.runner.handle_event(event);
    }
}

/// Create a script runner consumer with the specified configuration
pub fn create_script_consumer(
    action_tx: UnboundedSender<ClientAction>,
    config: &ScriptingConfig,
) -> ScriptConsumer {
    let runner = gromnie_scripting::create_runner_from_config(action_tx, config);
    ScriptConsumer::new(runner)
}
