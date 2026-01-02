use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::client_runner::MultiClientStats;
use crate::event_bus::{ClientStateEvent, EventEnvelope, EventType, ScriptEventType, SystemEvent};
use gromnie_client::client::{
    OutgoingMessageContent,
    events::{ClientAction, GameEvent},
};
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::time::Instant;

// Re-export EventConsumer from gromnie-events
pub use gromnie_events::EventConsumer;

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

    /// Create a factory for this consumer
    pub fn from_factory() -> impl crate::client_runner_builder::ConsumerFactory {
        LoggingConsumerFactory
    }
}

struct LoggingConsumerFactory;

impl crate::client_runner_builder::ConsumerFactory for LoggingConsumerFactory {
    fn create(
        &self,
        ctx: &crate::client_runner_builder::ConsumerContext,
    ) -> Box<dyn EventConsumer> {
        Box::new(LoggingConsumer::new(ctx.action_tx.clone()))
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
    tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
}

impl TuiConsumer {
    pub fn new(
        action_tx: UnboundedSender<ClientAction>,
        tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
    ) -> Self {
        Self {
            _action_tx: action_tx,
            tui_event_tx,
        }
    }

    /// Create a factory for this consumer
    pub fn from_factory(
        tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
    ) -> impl crate::client_runner_builder::ConsumerFactory {
        TuiConsumerFactory { tui_event_tx }
    }
}

struct TuiConsumerFactory {
    tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
}

impl crate::client_runner_builder::ConsumerFactory for TuiConsumerFactory {
    fn create(
        &self,
        ctx: &crate::client_runner_builder::ConsumerContext,
    ) -> Box<dyn EventConsumer> {
        Box::new(TuiConsumer::new(
            ctx.action_tx.clone(),
            self.tui_event_tx.clone(),
        ))
    }
}

impl EventConsumer for TuiConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            EventType::Game(game_event) => {
                tracing::info!(target: "tui_consumer", "TuiConsumer forwarding GameEvent: {:?}", std::mem::discriminant(&game_event));
                let _ = self.tui_event_tx.send(game_event.into());
            }
            EventType::System(system_event) => {
                tracing::info!(target: "tui_consumer", "TuiConsumer forwarding SystemEvent: {:?}", std::mem::discriminant(&system_event));
                let _ = self.tui_event_tx.send(system_event.into());
            }
            EventType::State(_) => {
                // State events are logged by other consumers, TUI doesn't need them
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

    /// Create a factory for this consumer
    pub fn from_factory(
        http: Arc<Http>,
        channel_id: ChannelId,
    ) -> impl crate::client_runner_builder::ConsumerFactory {
        DiscordConsumerFactory {
            http,
            channel_id,
            uptime_data: None,
        }
    }

    /// Create a factory for this consumer with uptime tracking
    pub fn from_factory_with_uptime(
        http: Arc<Http>,
        channel_id: ChannelId,
        uptime_data: Arc<tokio::sync::RwLock<UptimeData>>,
    ) -> impl crate::client_runner_builder::ConsumerFactory {
        DiscordConsumerFactory {
            http,
            channel_id,
            uptime_data: Some(uptime_data),
        }
    }
}

struct DiscordConsumerFactory {
    http: Arc<Http>,
    channel_id: ChannelId,
    uptime_data: Option<Arc<tokio::sync::RwLock<UptimeData>>>,
}

impl crate::client_runner_builder::ConsumerFactory for DiscordConsumerFactory {
    fn create(
        &self,
        ctx: &crate::client_runner_builder::ConsumerContext,
    ) -> Box<dyn EventConsumer> {
        if let Some(ref uptime_data) = self.uptime_data {
            Box::new(DiscordConsumer::new_with_uptime(
                ctx.action_tx.clone(),
                self.http.clone(),
                self.channel_id,
                uptime_data.clone(),
            ))
        } else {
            Box::new(DiscordConsumer::new(
                ctx.action_tx.clone(),
                self.http.clone(),
                self.channel_id,
            ))
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

/// Consumer that collects statistics across clients for multi-client runs
pub struct StatsConsumer {
    client_id: u32,
    stats: Arc<MultiClientStats>,
    verbose: bool,
}

impl StatsConsumer {
    /// Create a new stats consumer
    pub fn new(client_id: u32, stats: Arc<MultiClientStats>) -> Self {
        Self {
            client_id,
            stats,
            verbose: false,
        }
    }

    /// Enable verbose logging for this consumer
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Create a factory for this consumer
    pub fn from_factory(
        stats: Arc<MultiClientStats>,
        verbose: bool,
    ) -> impl crate::client_runner_builder::ConsumerFactory {
        StatsConsumerFactory { stats, verbose }
    }
}

struct StatsConsumerFactory {
    stats: Arc<MultiClientStats>,
    verbose: bool,
}

impl crate::client_runner_builder::ConsumerFactory for StatsConsumerFactory {
    fn create(
        &self,
        ctx: &crate::client_runner_builder::ConsumerContext,
    ) -> Box<dyn EventConsumer> {
        Box::new(StatsConsumer::new(ctx.client_id, self.stats.clone()).with_verbose(self.verbose))
    }
}

impl EventConsumer for StatsConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        if let Some(event) = envelope.extract_game_event() {
            match event {
                GameEvent::AuthenticationSucceeded => {
                    self.stats.authenticated.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        info!("[Client {}] Authentication succeeded", self.client_id);
                    }
                }
                GameEvent::LoginSucceeded { .. } => {
                    self.stats.logged_in.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        info!("[Client {}] Login succeeded", self.client_id);
                    }
                }
                GameEvent::AuthenticationFailed { .. } => {
                    self.stats.errors.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        error!("[Client {}] Authentication failed", self.client_id);
                    }
                }
                GameEvent::LoginFailed { .. } => {
                    self.stats.errors.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        error!("[Client {}] Login failed", self.client_id);
                    }
                }
                _ => {}
            }
        }
    }
}

/// State machine for auto-login consumer
#[derive(Clone, Debug, PartialEq)]
pub enum AutoLoginState {
    /// Waiting for character list, haven't found our character yet
    WaitingForCharList,
    /// Character not found in list, creation in progress
    CharacterCreationInProgress,
    /// Character found in list, ready to log in
    CharacterFound,
}

/// Consumer that automatically creates a character and logs in
///
/// This consumer implements the load tester behavior:
/// 1. Wait for CharacterListReceived
/// 2. If character doesn't exist, create it
/// 3. Log in with the character
pub struct AutoLoginConsumer {
    client_id: u32,
    character_name: String,
    action_tx: UnboundedSender<ClientAction>,
    state: AutoLoginState,
    verbose: bool,
}

impl AutoLoginConsumer {
    /// Create a new auto-login consumer
    ///
    /// # Arguments
    /// * `client_id` - The client ID for logging
    /// * `character_name` - The name of the character to create/login with
    /// * `action_tx` - Channel to send actions back to the client
    pub fn new(
        client_id: u32,
        character_name: String,
        action_tx: UnboundedSender<ClientAction>,
    ) -> Self {
        Self {
            client_id,
            character_name,
            action_tx,
            state: AutoLoginState::WaitingForCharList,
            verbose: false,
        }
    }

    /// Enable verbose logging for this consumer
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Get the current state
    pub fn state(&self) -> &AutoLoginState {
        &self.state
    }

    /// Create a factory for this consumer
    pub fn from_factory(
        character_name: String,
        verbose: bool,
    ) -> impl crate::client_runner_builder::ConsumerFactory {
        AutoLoginConsumerFactory {
            character_name,
            verbose,
        }
    }
}

struct AutoLoginConsumerFactory {
    character_name: String,
    verbose: bool,
}

impl crate::client_runner_builder::ConsumerFactory for AutoLoginConsumerFactory {
    fn create(
        &self,
        ctx: &crate::client_runner_builder::ConsumerContext,
    ) -> Box<dyn EventConsumer> {
        Box::new(
            AutoLoginConsumer::new(
                ctx.client_id,
                self.character_name.clone(),
                ctx.action_tx.clone(),
            )
            .with_verbose(self.verbose),
        )
    }
}

impl EventConsumer for AutoLoginConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        if let Some(GameEvent::CharacterListReceived {
            characters,
            account,
            ..
        }) = envelope.extract_game_event()
        {
            if self.verbose {
                info!(
                    "[Client {}] Got character list for {}: {} chars",
                    self.client_id,
                    account,
                    characters.len()
                );
            }

            // Handle based on current state
            match self.state {
                AutoLoginState::WaitingForCharList
                | AutoLoginState::CharacterCreationInProgress => {
                    // Check if our character exists
                    if let Some(char_info) =
                        characters.iter().find(|c| c.name == self.character_name)
                    {
                        // Character found (either was there initially or just created)
                        if self.verbose {
                            info!(
                                "[Client {}] Found character: {} (ID: {})",
                                self.client_id, char_info.name, char_info.id
                            );
                        }
                        // Update state and proceed to login
                        self.state = AutoLoginState::CharacterFound;
                        if let Err(e) = self.action_tx.send(ClientAction::LoginCharacter {
                            character_id: char_info.id,
                            character_name: char_info.name.clone(),
                            account: account.clone(),
                        }) {
                            error!(
                                "[Client {}] Failed to send login action: {}",
                                self.client_id, e
                            );
                        }
                    } else if self.state == AutoLoginState::WaitingForCharList {
                        // Character doesn't exist yet - create it
                        if self.verbose {
                            info!(
                                "[Client {}] Creating character: {}",
                                self.client_id, self.character_name
                            );
                        }
                        self.state = AutoLoginState::CharacterCreationInProgress;

                        let char_gen_result =
                            crate::CharacterBuilder::new(self.character_name.clone()).build();
                        let msg = OutgoingMessageContent::CharacterCreationAce(
                            account.clone(),
                            char_gen_result,
                        );
                        if let Err(e) = self
                            .action_tx
                            .send(ClientAction::SendMessage(Box::new(msg)))
                        {
                            error!(
                                "[Client {}] Failed to send character creation: {}",
                                self.client_id, e
                            );
                        }
                    }
                }
                AutoLoginState::CharacterFound => {
                    // Already found and logging in, ignore further character list updates
                    if self.verbose {
                        info!(
                            "[Client {}] Already processing login, ignoring character list update",
                            self.client_id
                        );
                    }
                }
            }
        }
    }
}

/// Consumer that composes multiple consumers together
///
/// This allows chaining multiple consumers to handle different aspects
/// of event processing (e.g., stats + auto-login).
pub struct CompositeConsumer {
    consumers: Vec<Box<dyn EventConsumer>>,
}

impl CompositeConsumer {
    /// Create a new composite consumer
    pub fn new(consumers: Vec<Box<dyn EventConsumer>>) -> Self {
        Self { consumers }
    }

    /// Add a consumer to the composite
    pub fn with_consumer(mut self, consumer: Box<dyn EventConsumer>) -> Self {
        self.consumers.push(consumer);
        self
    }
}

impl EventConsumer for CompositeConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        for consumer in &mut self.consumers {
            consumer.handle_event(envelope.clone());
        }
    }
}
