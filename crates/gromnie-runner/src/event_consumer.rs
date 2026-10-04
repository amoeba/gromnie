use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::client_runner::MultiClientStats;
use crate::event_bus::{EventEnvelope, EventType, SystemEvent};
use gromnie_client::client::ClientSender;
use gromnie_events::{ProtocolEvent, S2CEvent};
use serenity::http::Http;
use serenity::model::id::ChannelId;

/// Format a Duration as (hours, minutes, seconds)
fn format_uptime(duration: Duration) -> (u64, u64, u64) {
    let secs = duration.as_secs();
    (secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// Log protocol events shared between LoggingConsumer and DiscordConsumer.
/// Returns true if the event was handled.
fn log_common_protocol_event(event: &ProtocolEvent) -> bool {
    match event {
        ProtocolEvent::S2C(S2CEvent::LoginCharacterSet {
            account,
            characters,
            num_slots,
        }) => {
            let names = characters
                .iter()
                .map(|c| format!("{} ({})", c.name, c.character_id.0))
                .collect::<Vec<_>>()
                .join(", ");
            info!(target: "events", "CharacterList -- Account: {}, Slots: {}, Number of Chars: {}, Chars: {}", account, num_slots, characters.len(), names);
            true
        }
        ProtocolEvent::S2C(S2CEvent::CharacterError {
            error_code,
            error_message,
        }) => {
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
        SystemEvent::ConnectingProgress { progress, .. } => {
            debug!(target: "events", "Connecting progress: {:.1}%", progress * 100.0);
            true
        }
        SystemEvent::UpdatingProgress { progress, .. } => {
            debug!(target: "events", "Updating progress: {:.1}%", progress * 100.0);
            true
        }
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
    _sender: ClientSender,
}

impl LoggingConsumer {
    pub fn new(sender: ClientSender) -> Self {
        Self { _sender: sender }
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
        Box::new(LoggingConsumer::new(ctx.sender.clone()))
    }
}

impl EventConsumer for LoggingConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            EventType::Protocol(protocol_event) => {
                if log_common_protocol_event(&protocol_event) {
                    return;
                }
                // Chat arrives under several protocol messages; `as_chat_message`
                // normalizes them so log output is identical to what UIs show.
                if let Some(chat) = protocol_event.as_chat_message() {
                    info!(target: "events", "CHAT [{}]: {}", chat.message_type, chat.text);
                    return;
                }
                if let ProtocolEvent::S2C(s2c) = &protocol_event {
                    match s2c {
                        S2CEvent::LoginCreatePlayer { character_id } => {
                            info!(target: "events", "CREATE PLAYER: Character ID {}", character_id);
                        }
                        S2CEvent::ItemCreateObject {
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
                        S2CEvent::ItemOnViewContents {
                            container_id,
                            items,
                        } => {
                            info!(target: "events", "ITEM VIEW CONTENTS: Container {} has {} items", container_id, items.len());
                        }
                        S2CEvent::PlayerContainersReceived {
                            player_id,
                            containers,
                        } => {
                            info!(target: "events", "PLAYER CONTAINERS: Player {} has {} containers", player_id, containers.len());
                        }
                        S2CEvent::ItemDeleteObject { object_id } => {
                            info!(target: "events", "ITEM DELETE: Object ID {}", object_id);
                        }
                        S2CEvent::QualitiesPrivateUpdateInt { property, value } => {
                            info!(target: "events", "QUALITY UPDATE: property {} = {}", property, value);
                        }
                        S2CEvent::ItemSetState { object_id, state } => {
                            info!(target: "events", "ITEM SET STATE: Object {} state = {}", object_id, state);
                        }
                        _ => {}
                    }
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
    _sender: ClientSender,
    tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
}

impl TuiConsumer {
    pub fn new(
        sender: ClientSender,
        tui_event_tx: UnboundedSender<crate::event_bus::TuiEvent>,
    ) -> Self {
        Self {
            _sender: sender,
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
            ctx.sender.clone(),
            self.tui_event_tx.clone(),
        ))
    }
}

impl EventConsumer for TuiConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            EventType::Protocol(protocol_event) => {
                tracing::info!(target: "tui_consumer", "TuiConsumer forwarding ProtocolEvent: {:?}", std::mem::discriminant(&protocol_event));
                let _ = self.tui_event_tx.send(protocol_event.into());
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
    _sender: ClientSender,
    http: Arc<Http>,
    channel_id: ChannelId,
    bot_start_time: Instant,
    ingame_start_time: Option<Instant>,
    uptime_data: Option<Arc<tokio::sync::RwLock<UptimeData>>>,
}

impl DiscordConsumer {
    pub fn new(sender: ClientSender, http: Arc<Http>, channel_id: ChannelId) -> Self {
        Self {
            _sender: sender,
            http,
            channel_id,
            bot_start_time: Instant::now(),
            ingame_start_time: None,
            uptime_data: None,
        }
    }

    pub fn new_with_uptime(
        sender: ClientSender,
        http: Arc<Http>,
        channel_id: ChannelId,
        uptime_data: Arc<tokio::sync::RwLock<UptimeData>>,
    ) -> Self {
        Self {
            _sender: sender,
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
                ctx.sender.clone(),
                self.http.clone(),
                self.channel_id,
                uptime_data.clone(),
            ))
        } else {
            Box::new(DiscordConsumer::new(
                ctx.sender.clone(),
                self.http.clone(),
                self.channel_id,
            ))
        }
    }
}

impl EventConsumer for DiscordConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        match envelope.event {
            EventType::Protocol(protocol_event) => {
                if log_common_protocol_event(&protocol_event) {
                    return;
                }
                if let Some(chat) = protocol_event.as_chat_message() {
                    if let Some(ingame_start) = self.ingame_start_time {
                        let (h, m, s) = format_uptime(ingame_start.elapsed());
                        info!(target: "events", "CHAT [{}]: {} | In-game: {:02}:{:02}:{:02}", chat.message_type, chat.text, h, m, s);
                    } else {
                        info!(target: "events", "CHAT [{}]: {}", chat.message_type, chat.text);
                    }

                    let discord_message = format!("[{}] {}", chat.message_type, chat.text);
                    let http = self.http.clone();
                    let channel_id = self.channel_id;

                    tokio::spawn(async move {
                        if let Err(e) = channel_id.say(&http, &discord_message).await {
                            error!("Failed to send Discord message: {}", e);
                        }
                    });
                }
                // Everything else (inventory, movement, trade) is not chat and
                // is intentionally ignored here.
            }
            EventType::State(state_event) => {
                info!(target: "events", "STATE CHANGE: {:?}", state_event);
            }
            EventType::System(system_event) => {
                if log_common_system_event(&system_event) {
                    return;
                }
                if let SystemEvent::LoginSucceeded {
                    character_id,
                    character_name,
                } = system_event
                {
                    self.handle_login_succeeded(character_id, &character_name);
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
            EventType::Protocol(event) => {
                if let ProtocolEvent::S2C(S2CEvent::CharacterError { .. }) = event {
                    self.stats.errors.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        error!("[Client {}] Character error", self.client_id);
                    }
                }
            }
            EventType::System(event) => match event {
                SystemEvent::LoginSucceeded { .. } => {
                    self.stats.logged_in.fetch_add(1, Ordering::SeqCst);
                    if self.verbose {
                        info!("[Client {}] Login succeeded", self.client_id);
                    }
                }
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
/// 1. Wait for `LoginCharacterSet`
/// 2. If character doesn't exist, create it
/// 3. Log in with the character
pub struct AutoLoginConsumer {
    client_id: u32,
    character_name: String,
    sender: ClientSender,
    state: AutoLoginState,
    verbose: bool,
}

impl AutoLoginConsumer {
    /// Create a new auto-login consumer
    ///
    /// # Arguments
    /// * `client_id` - The client ID for logging
    /// * `character_name` - The name of the character to create/login with
    /// * `sender` - Channel to send actions back to the client
    pub fn new(client_id: u32, character_name: String, sender: ClientSender) -> Self {
        Self {
            client_id,
            character_name,
            sender,
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
                ctx.sender.clone(),
            )
            .with_verbose(self.verbose),
        )
    }
}

impl EventConsumer for AutoLoginConsumer {
    fn handle_event(&mut self, envelope: EventEnvelope) {
        if let Some(ProtocolEvent::S2C(S2CEvent::LoginCharacterSet {
            characters,
            account,
            ..
        })) = envelope.extract_protocol_event()
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
                        if let Err(e) = self.sender.enter_world(
                            char_info.character_id.0,
                            char_info.name.clone(),
                            account.clone(),
                        ) {
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
