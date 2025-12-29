use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::client::events::{CharacterInfo, ClientAction, GameEvent};
use crate::config::ScriptingConfig;
use crate::scripting::ScriptRunner;
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
    action_tx: UnboundedSender<ClientAction>,
    character_created: Arc<AtomicBool>,
    character_config: Option<String>,
}

impl LoggingConsumer {
    pub fn new(action_tx: UnboundedSender<ClientAction>) -> Self {
        Self {
            action_tx,
            character_created: Arc::new(AtomicBool::new(false)),
            character_config: Some("*".to_string()),
        }
    }

    pub fn new_with_character(
        action_tx: UnboundedSender<ClientAction>,
        character_config: Option<String>,
    ) -> Self {
        Self {
            action_tx,
            character_created: Arc::new(AtomicBool::new(false)),
            character_config,
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
                handle_character_list(
                    &account,
                    &characters,
                    num_slots,
                    &self.action_tx,
                    &self.character_created,
                    self.character_config.as_deref(),
                );
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
    action_tx: UnboundedSender<ClientAction>,
    tui_event_tx: UnboundedSender<GameEvent>,
    character_created: Arc<AtomicBool>,
    character_config: Option<String>,
}

impl TuiConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        tui_event_tx: UnboundedSender<GameEvent>,
    ) -> Self {
        Self {
            action_tx,
            tui_event_tx,
            character_created: Arc::new(AtomicBool::new(false)),
            character_config: Some("*".to_string()),
        }
    }

    pub fn new_with_character(
        action_tx: UnboundedSender<ClientAction>,
        tui_event_tx: UnboundedSender<GameEvent>,
        character_config: Option<String>,
    ) -> Self {
        Self {
            action_tx,
            tui_event_tx,
            character_created: Arc::new(AtomicBool::new(false)),
            character_config,
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
                handle_character_list(
                    &account,
                    &characters,
                    num_slots,
                    &self.action_tx,
                    &self.character_created,
                    self.character_config.as_deref(),
                );
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
    action_tx: UnboundedSender<ClientAction>,
    http: Arc<Http>,
    channel_id: ChannelId,
    character_created: Arc<AtomicBool>,
    bot_start_time: Instant,
    ingame_start_time: Option<Instant>,
    uptime_data: Option<Arc<tokio::sync::RwLock<UptimeData>>>,
    character_config: Option<String>,
}

impl DiscordConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        http: Arc<Http>,
        channel_id: ChannelId,
    ) -> Self {
        Self {
            action_tx,
            http,
            channel_id,
            character_created: Arc::new(AtomicBool::new(false)),
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: None,
            character_config: Some("*".to_string()),
        }
    }

    pub fn new_with_uptime(
        action_tx: UnboundedSender<ClientAction>,
        http: Arc<Http>,
        channel_id: ChannelId,
        uptime_data: Arc<tokio::sync::RwLock<UptimeData>>,
    ) -> Self {
        Self {
            action_tx,
            http,
            channel_id,
            character_created: Arc::new(AtomicBool::new(false)),
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: Some(uptime_data),
            character_config: Some("*".to_string()),
        }
    }

    pub fn new_with_character(
        action_tx: UnboundedSender<ClientAction>,
        http: Arc<Http>,
        channel_id: ChannelId,
        uptime_data: Arc<tokio::sync::RwLock<UptimeData>>,
        character_config: Option<String>,
    ) -> Self {
        Self {
            action_tx,
            http,
            channel_id,
            character_created: Arc::new(AtomicBool::new(false)),
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: Some(uptime_data),
            character_config,
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
                handle_character_list(
                    &account,
                    &characters,
                    num_slots,
                    &self.action_tx,
                    &self.character_created,
                    self.character_config.as_deref(),
                );
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

/// Shared logic for handling character list received event
///
/// Auto-login behavior based on character_config:
/// - If character_config is Some(name): Login to that specific character (error if not found)
/// - If character_config is Some("*"): Login to any character (first one)
/// - If character_config is None: Error (must specify character)
/// - If no characters exist: Always error
pub fn handle_character_list(
    account: &str,
    characters: &[CharacterInfo],
    num_slots: u32,
    action_tx: &UnboundedSender<ClientAction>,
    character_created: &Arc<AtomicBool>,
    character_config: Option<&str>,
) {
    let names = characters
        .iter()
        .map(|c| format!("{} ({})", c.name, c.id))
        .collect::<Vec<_>>()
        .join(", ");
    info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);

    // Check if already handled
    if character_created.load(Ordering::SeqCst) {
        return;
    }

    // If no characters exist, error
    if characters.is_empty() {
        error!(target: "events", "No characters found on account '{}'. Please create a character first.", account);
        error!(target: "events", "Character creation is not yet supported. Use the official client to create a character.");
        character_created.store(true, Ordering::SeqCst);
        return;
    }

    // Characters exist - determine which one to login
    let character_to_login = match character_config {
        None | Some("") => {
            error!(target: "events", "Character must be specified in account config. Set 'character' to a character name or \"*\" for any.");
            character_created.store(true, Ordering::SeqCst);
            return;
        }
        Some("*") => {
            // Login to first character
            info!(target: "events", "Auto-login: Using first available character");
            &characters[0]
        }
        Some(char_name) => {
            // Find specific character
            match characters.iter().find(|c| c.name == char_name) {
                Some(char) => {
                    info!(target: "events", "Auto-login: Found requested character '{}'", char_name);
                    char
                }
                None => {
                    error!(target: "events", "Character '{}' not found on account '{}'. Available characters: {}", char_name, account, names);
                    character_created.store(true, Ordering::SeqCst);
                    return;
                }
            }
        }
    };

    info!(target: "events", "Attempting to log in as: {} (ID: {})", character_to_login.name, character_to_login.id);

    // Send action to login
    if let Err(e) = action_tx.send(ClientAction::LoginCharacter {
        character_id: character_to_login.id,
        character_name: character_to_login.name.clone(),
        account: account.to_string(),
    }) {
        error!(target: "events", "Failed to send login action: {}", e);
    } else {
        character_created.store(true, Ordering::SeqCst);
    }
}

/// Auto-login consumer that handles character selection and login
pub struct AutoLoginConsumer {
    action_tx: UnboundedSender<ClientAction>,
    character_created: Arc<AtomicBool>,
    character_config: Option<String>,
}

impl AutoLoginConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        character_config: Option<String>,
    ) -> Self {
        Self {
            action_tx,
            character_created: Arc::new(AtomicBool::new(false)),
            character_config,
        }
    }
}

impl EventConsumer for AutoLoginConsumer {
    fn handle_event(&mut self, event: GameEvent) {
        // Only handle character list events
        if let GameEvent::CharacterListReceived {
            ref account,
            ref characters,
            num_slots,
        } = event
        {
            handle_character_list(
                account,
                characters,
                num_slots,
                &self.action_tx,
                &self.character_created,
                self.character_config.as_deref(),
            );
        }
    }
}

/// Create a script runner consumer with the specified configuration
pub fn create_script_consumer(
    action_tx: UnboundedSender<ClientAction>,
    config: &ScriptingConfig,
) -> ScriptRunner {
    let registry = crate::scripts::create_registry();
    registry.create_runner(action_tx, &config.enabled_scripts)
}
