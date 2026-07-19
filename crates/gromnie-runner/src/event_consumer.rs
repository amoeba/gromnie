use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::client_runner::MultiClientStats;
use crate::event_bus::{EventEnvelope, EventType, SystemEvent};
use gromnie_events::{SimpleClientAction, SimpleGameEvent};
use serenity::http::Http;
use serenity::model::id::ChannelId;

// Alias for backward compatibility in this file
use SimpleGameEvent as GameEvent;

/// Format a Duration as (hours, minutes, seconds)
fn format_uptime(duration: Duration) -> (u64, u64, u64) {
    let secs = duration.as_secs();
    (secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// Log game events shared between LoggingConsumer and DiscordConsumer.
/// Returns true if the event was handled.
fn log_common_game_event(event: &GameEvent) -> bool {
    match event {
        GameEvent::CharacterListReceived {
            account,
            characters,
            num_slots,
        } => {
            let names = characters
                .iter()
                .map(|c| format!("{} ({})", c.name, c.character_id.0))
                .collect::<Vec<_>>()
                .join(", ");
            info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);
            true
        }
        GameEvent::LoginFailed { reason } => {
            error!(target: "events", "LoginFailed -- Reason: {}", reason);
            true
        }
        GameEvent::ConnectingSetProgress { progress } => {
            debug!(target: "events", "Connecting progress: {:.1}%", progress * 100.0);
            true
        }
        GameEvent::UpdatingSetProgress { progress } => {
            debug!(target: "events", "Updating progress: {:.1}%", progress * 100.0);
            true
        }
        GameEvent::CharacterError {
            error_code,
            error_message,
        } => {
            error!(target: "events", "Character error (code {}): {}", error_code, error_message);
            true
        }
        _ => false,
    }
}

/// Log system events shared between LoggingConsumer and DiscordConsumer.
/// Returns true if the event was handled.
fn log_common_system_event(event: &SystemEvent) -> bool {
    match event {
        SystemEvent::AuthenticationSucceeded { .. } => {
            info!(target: "events", "Authentication succeeded - connected to server");
            true
        }
        SystemEvent::AuthenticationFailed { reason, .. } => {
            error!(target: "events", "Authentication failed: {}", reason);
            true
        }
        SystemEvent::Disconnected {
            will_reconnect,
            reconnect_attempt,
            delay_secs,
            ..
        } => {
            info!(
                target: "events",
                "Disconnected (will_reconnect={}, attempt={}, delay={}s)",
                will_reconnect, reconnect_attempt, delay_secs
            );
            true
        }
        SystemEvent::Reconnecting {
            attempt,
            delay_secs,
            ..
        } => {
            info!(target: "events", "Reconnecting (attempt={}, delay={}s)", attempt, delay_secs);
            true
        }
        _ => false,
    }
}

// Re-export EventConsumer from gromnie-events
pub use gromnie_events::EventConsumer;

/// Event consumer that logs events to the console (for CLI version)
pub struct LoggingConsumer {
    _action_tx: UnboundedSender<SimpleClientAction>,
}

impl LoggingConsumer {
    pub fn new(action_tx: UnboundedSender<SimpleClientAction>) -> Self {
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
        match envelope.event {
            EventType::Game(game_event) => {
                if log_common_game_event(&game_event) {
                    return;
                }
                match game_event {
                    GameEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
                    }
                    GameEvent::ChatMessageReceived {
                        message,
                        message_type,
                    } => {
                        info!(target: "events", "CHAT [{}]: {}", message_type, message);
                    }
                    GameEvent::CreatePlayer { character_id } => {
                        info!(target: "events", "CREATE PLAYER: Character ID {}", character_id);
                    }
                    GameEvent::ItemCreateObject {
                        object_id,
                        name,
                        item_type,
                        container_id,
                        burden,
                        value,
                        items_capacity: _,
                        container_capacity: _,
                    } => {
                        info!(target: "events", "ITEM CREATE: {} (ID: {}, Type: {}, Container: {:?}, Burden: {}, Value: {})",
                            name, object_id, item_type, container_id, burden, value);
                    }
                    GameEvent::ItemOnViewContents {
                        container_id,
                        items,
                    } => {
                        info!(target: "events", "ITEM VIEW CONTENTS: Container {} has {} items", container_id, items.len());
                    }
                    GameEvent::PlayerContainersReceived {
                        player_id,
                        containers,
                    } => {
                        info!(target: "events", "PLAYER CONTAINERS: Player {} has {} containers", player_id, containers.len());
                    }
                    GameEvent::ItemDeleteObject { object_id } => {
                        info!(target: "events", "ITEM DELETE: Object ID {}", object_id);
                    }
                    GameEvent::ItemMovedObject {
                        object_id,
                        new_container_id,
                    } => {
                        info!(target: "events", "ITEM MOVED: Object {} moved to container {}", object_id, new_container_id);
                    }
                    GameEvent::QualitiesPrivateUpdateInt {
                        object_id,
                        property_name,
                        value,
                    } => {
                        info!(target: "events", "QUALITY UPDATE: Object {} property {} = {}", object_id, property_name, value);
                    }
                    GameEvent::ItemSetState {
                        object_id,
                        property_name,
                        value,
                    } => {
                        info!(target: "events", "ITEM SET STATE: Object {} property {} = {}", object_id, property_name, value);
                    }
                    _ => {}
                }
            }
            EventType::State(state_event) => {
                info!(target: "events", "STATE CHANGE: {:?}", state_event);
            }
            EventType::System(system_event) => {
                if log_common_system_event(&system_event) {
                    return;
                }
                match system_event {
                    SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
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
                    SystemEvent::ReloadScripts { .. } => {
                        info!(target: "events", "Reloading scripts");
                    }
                    SystemEvent::LogScriptMessage { script_id, message } => {
                        info!(target: "events", "Script [{}]: {}", script_id, message);
                    }
                    SystemEvent::Shutdown => {
                        info!(target: "events", "System shutdown");
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Event consumer that forwards events to TUI and logs to console
pub struct TuiConsumer {
    _action_tx: UnboundedSender<SimpleClientAction>,
    tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
}

impl TuiConsumer {
    pub fn new(
        action_tx: UnboundedSender<SimpleClientAction>,
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
            EventType::State(state_event) => {
                tracing::info!(target: "tui_consumer", "TuiConsumer forwarding StateEvent: {:?}", std::mem::discriminant(&state_event));
                let _ = self.tui_event_tx.send(state_event.into());
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

impl UptimeData {
    pub fn format_bot_uptime(&self) -> String {
        let (h, m, s) = format_uptime(self.bot_start.elapsed());
        format!("{:02}:{:02}:{:02}", h, m, s)
    }

    pub fn format_ingame_uptime(&self) -> Option<String> {
        self.ingame_start.map(|start| {
            let (h, m, s) = format_uptime(start.elapsed());
            format!("{:02}:{:02}:{:02}", h, m, s)
        })
    }
}

/// Event consumer that forwards chat messages to Discord
pub struct DiscordConsumer {
    _action_tx: UnboundedSender<SimpleClientAction>,
    http: Arc<Http>,
    channel_id: ChannelId,
    bot_start_time: Instant,
    ingame_start_time: Option<Instant>,
    uptime_data: Option<Arc<tokio::sync::RwLock<UptimeData>>>,
}

impl DiscordConsumer {
    pub fn new(
        action_tx: UnboundedSender<SimpleClientAction>,
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
        action_tx: UnboundedSender<SimpleClientAction>,
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

    fn handle_login_succeeded(&mut self, character_id: u32, character_name: &str) {
        let now = Instant::now();
        self.ingame_start_time = Some(now);

        if let Some(ref uptime_data) = self.uptime_data {
            let uptime_data_clone = uptime_data.clone();
            tokio::spawn(async move {
                let mut data = uptime_data_clone.write().await;
                data.ingame_start = Some(now);
            });
        }

        let (h, m, s) = format_uptime(self.bot_start_time.elapsed());

        info!(target: "events", "LoginSucceeded -- Character: {} (ID: {})", character_name, character_id);
        info!(target: "events", "Bot uptime: {:02}:{:02}:{:02} | Now tracking in-game time", h, m, s);
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
        match envelope.event {
            EventType::Game(game_event) => {
                if log_common_game_event(&game_event) {
                    return;
                }
                match game_event {
                    GameEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        self.handle_login_succeeded(character_id, &character_name);
                    }
                    GameEvent::ChatMessageReceived {
                        message,
                        message_type,
                    } => {
                        if let Some(ingame_start) = self.ingame_start_time {
                            let (h, m, s) = format_uptime(ingame_start.elapsed());
                            info!(target: "events", "CHAT [{}]: {} | In-game: {:02}:{:02}:{:02}", message_type, message, h, m, s);
                        } else {
                            info!(target: "events", "CHAT [{}]: {}", message_type, message);
                        }

                        let discord_message = format!("[{}] {}", message_type, message);
                        let http = self.http.clone();
                        let channel_id = self.channel_id;

                        tokio::spawn(async move {
                            if let Err(e) = channel_id.say(&http, &discord_message).await {
                                error!("Failed to send Discord message: {}", e);
                            }
                        });
                    }
                    GameEvent::CreatePlayer { character_id } => {
                        debug!(target: "events", "CREATE PLAYER: Character ID {}", character_id);
                    }
                    GameEvent::ItemCreateObject { .. }
                    | GameEvent::ItemOnViewContents { .. }
                    | GameEvent::PlayerContainersReceived { .. }
                    | GameEvent::ItemDeleteObject { .. }
                    | GameEvent::ItemMovedObject { .. }
                    | GameEvent::QualitiesPrivateUpdateInt { .. }
                    | GameEvent::ItemSetState { .. } => {
                        // Ignore inventory events in Discord consumer
                    }
                    _ => {}
                }
            }
            EventType::State(state_event) => {
                info!(target: "events", "STATE CHANGE: {:?}", state_event);
            }
            EventType::System(system_event) => {
                if log_common_system_event(&system_event) {
                    return;
                }
                match system_event {
                    SystemEvent::LoginSucceeded {
                        character_id,
                        character_name,
                    } => {
                        self.handle_login_succeeded(character_id, &character_name);
                    }
                    _ => {}
                }
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
        match envelope.event {
            EventType::Game(event) => match event {
                GameEvent::LoginSucceeded { .. } => {
                    self.stats.logged_in.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        info!("[Client {}] Login succeeded", self.client_id);
                    }
                }
                GameEvent::LoginFailed { .. } => {
                    self.stats.errors.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        error!("[Client {}] Login failed", self.client_id);
                    }
                }
                _ => {}
            },
            EventType::System(event) => match event {
                SystemEvent::AuthenticationSucceeded { .. } => {
                    self.stats.authenticated.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        info!("[Client {}] Authentication succeeded", self.client_id);
                    }
                }
                SystemEvent::AuthenticationFailed { .. } => {
                    self.stats.errors.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        error!("[Client {}] Authentication failed", self.client_id);
                    }
                }
                _ => {}
            },
            EventType::State(_) => {}
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
    action_tx: UnboundedSender<SimpleClientAction>,
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
        action_tx: UnboundedSender<SimpleClientAction>,
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
                                self.client_id, char_info.name, char_info.character_id.0
                            );
                        }
                        // Update state and proceed to login
                        self.state = AutoLoginState::CharacterFound;
                        if let Err(e) = self.action_tx.send(SimpleClientAction::LoginCharacter {
                            character_id: char_info.character_id.0,
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

                        // TODO: Need to implement character creation action
                        // For now, just log that we would create a character
                        if self.verbose {
                            info!(
                                "[Client {}] Would create character: {} in account {}",
                                self.client_id, self.character_name, account
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
