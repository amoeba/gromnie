use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::info;

use super::timer::TimerId;
use asheron_rs::message::GameActionMessage;
use gromnie_client::client::{Client, ClientSender};

/// Client state snapshot for scripts (clones of session and scene state)
#[derive(Debug, Clone)]
pub struct ClientState {
    pub session: gromnie_client::client::ClientSession,
    pub scene: gromnie_client::client::Scene,
}

/// Snapshot of client state at the time of an event (deprecated - use ClientState instead)
#[derive(Debug, Clone)]
pub struct ClientStateSnapshot {
    /// Current character ID (if logged in)
    pub character_id: Option<u32>,
    /// Current character name (if logged in)
    pub character_name: Option<String>,
    /// Whether we're currently in the game world
    pub is_ingame: bool,
    /// Whether we're authenticated with the server
    pub is_authenticated: bool,
}

impl ClientStateSnapshot {
    /// Create a new empty state snapshot
    pub fn new() -> Self {
        Self {
            character_id: None,
            character_name: None,
            is_ingame: false,
            is_authenticated: false,
        }
    }
}

impl Default for ClientStateSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Context provided to scripts for interacting with the client
pub struct ScriptContext {
    /// Shared reference to the client
    client: Arc<RwLock<Client>>,
    /// Channels for driving the client: session commands and game actions
    sender: ClientSender,
    /// Shared timer manager
    timer_manager: Arc<super::timer::TimerManager>,
    /// Timestamp when the current event occurred
    event_time: SystemTime,
}

impl ScriptContext {
    /// Create a new script context
    pub(crate) async fn new(
        client: Arc<RwLock<Client>>,
        sender: ClientSender,
        timer_manager: Arc<super::timer::TimerManager>,
        event_time: SystemTime,
    ) -> Self {
        Self {
            client,
            sender,
            timer_manager,
            event_time,
        }
    }

    /// Get current client state (clones session and scene from the client)
    pub async fn client(&self) -> ClientState {
        let client_guard = self.client.read().await;
        ClientState {
            session: client_guard.session.clone(),
            scene: client_guard.scene.clone(),
        }
    }

    /// Get current client state synchronously (uses try_read)
    pub fn client_sync(&self) -> ClientState {
        let client_guard = self
            .client
            .try_read()
            .expect("client lock should not be contended during script event handling");
        ClientState {
            session: client_guard.session.clone(),
            scene: client_guard.scene.clone(),
        }
    }

    /// Get the shared client handle for callers that need to hold it across await boundaries.
    pub fn client_arc(&self) -> Arc<RwLock<Client>> {
        Arc::clone(&self.client)
    }

    // ===== Action Methods =====

    /// Send a chat message (say to nearby players)
    pub fn send_chat(&self, message: impl Into<String>) {
        let _ = self.sender.say(message);
    }

    /// Send a direct message to a specific player
    pub fn send_tell(&self, recipient: impl Into<String>, message: impl Into<String>) {
        let _ = self.sender.tell(recipient, message);
    }

    /// Send a raw game action, for anything without a typed helper above.
    pub fn send_game_action(&self, action: GameActionMessage) {
        let _ = self.sender.send_game_action(action);
    }

    /// Request entry into the world for a character.
    pub fn login_character(
        &self,
        character_id: u32,
        character_name: impl Into<String>,
        account: impl Into<String>,
    ) {
        let _ = self
            .sender
            .enter_world(character_id, character_name, account);
    }

    /// Send a movement command to the server.
    ///
    /// `hold_key` is a `HoldKey` discriminant: `0` is Invalid, `1` None, `2` Run.
    pub fn do_movement_command(&self, motion: u32, speed: f32, hold_key: u32) {
        let _ = self.sender.do_movement_command(motion, speed, hold_key);
    }

    /// Stop a movement command. `motion` and `hold_key` must match the
    /// originating [`do_movement_command`](Self::do_movement_command).
    pub fn stop_movement_command(&self, motion: u32, hold_key: u32) {
        let _ = self.sender.stop_movement_command(motion, hold_key);
    }

    /// Write a script log line through the `script` tracing target.
    pub fn log_script_message(&self, script_id: &str, message: &str) {
        info!(target: "script", "[{}] {}", script_id, message);
    }

    // ===== Trading =====

    pub fn open_trade(&self, partner_id: u32) {
        use asheron_rs::gameactions::TradeOpenTradeNegotiations;
        use asheron_rs::types::ObjectId;
        self.send_game_action(GameActionMessage::TradeOpenTradeNegotiations(
            TradeOpenTradeNegotiations {
                object_id: ObjectId(partner_id),
            },
        ));
    }

    pub fn add_to_trade(&self, item_id: u32, slot: u32) {
        use asheron_rs::gameactions::TradeAddToTrade;
        use asheron_rs::types::ObjectId;
        self.send_game_action(GameActionMessage::TradeAddToTrade(TradeAddToTrade {
            object_id: ObjectId(item_id),
            slot_index: slot,
        }));
    }

    pub fn accept_trade(&self) {
        use asheron_rs::gameactions::TradeAcceptTrade;
        use asheron_rs::types::{ObjectId, Trade};
        let client = self
            .client
            .try_read()
            .expect("client lock should not be contended during accept_trade");
        let Some(trade) = client.pending_trade() else {
            tracing::warn!(target: "scripting", "accept_trade called but no pending trade");
            return;
        };
        self.send_game_action(GameActionMessage::TradeAcceptTrade(TradeAcceptTrade {
            contents: Trade {
                partner_id: ObjectId(trade.partner_id),
                sequence: trade.stamp as u64,
                status: 0,
                initiator_id: ObjectId(trade.initiator_id),
                accepted: true,
                partner_accepted: false,
            },
        }));
    }

    pub fn decline_trade(&self) {
        use asheron_rs::gameactions::TradeDeclineTrade;
        self.send_game_action(GameActionMessage::TradeDeclineTrade(TradeDeclineTrade {}));
    }

    pub fn reset_trade(&self) {
        use asheron_rs::gameactions::TradeResetTrade;
        self.send_game_action(GameActionMessage::TradeResetTrade(TradeResetTrade {}));
    }

    pub fn close_trade(&self) {
        use asheron_rs::gameactions::TradeCloseTradeNegotiations;
        self.send_game_action(GameActionMessage::TradeCloseTradeNegotiations(
            TradeCloseTradeNegotiations {},
        ));
    }

    // ===== Spell Casting =====

    pub fn cast_targeted_spell(&self, target_id: u32, spell_id: u32) {
        use asheron_rs::gameactions::MagicCastTargetedSpell;
        use asheron_rs::types::{LayeredSpellId, ObjectId, SpellId};
        self.send_game_action(GameActionMessage::MagicCastTargetedSpell(
            MagicCastTargetedSpell {
                object_id: ObjectId(target_id),
                spell_id: LayeredSpellId {
                    id: SpellId(spell_id as u16),
                    layer: 0,
                },
            },
        ));
    }

    pub fn cast_untargeted_spell(&self, spell_id: u32) {
        use asheron_rs::gameactions::MagicCastUntargetedSpell;
        use asheron_rs::types::{LayeredSpellId, SpellId};
        self.send_game_action(GameActionMessage::MagicCastUntargetedSpell(
            MagicCastUntargetedSpell {
                spell_id: LayeredSpellId {
                    id: SpellId(spell_id as u16),
                    layer: 0,
                },
            },
        ));
    }

    // ===== Timer Methods =====

    /// Schedule a one-shot timer that fires after a delay
    pub fn schedule_timer(&self, delay_secs: u64, name: impl Into<String>) -> TimerId {
        self.timer_manager
            .schedule_timer(Duration::from_secs(delay_secs), name.into())
    }

    /// Schedule a recurring timer that fires repeatedly at an interval
    pub fn schedule_recurring(&self, interval_secs: u64, name: impl Into<String>) -> TimerId {
        self.timer_manager
            .schedule_recurring(Duration::from_secs(interval_secs), name.into())
    }

    /// Cancel a timer
    pub fn cancel_timer(&self, timer_id: TimerId) -> bool {
        self.timer_manager.cancel_timer(timer_id)
    }

    /// Check if a timer has fired (consumes the fired state)
    pub fn check_timer(&self, timer_id: TimerId) -> bool {
        self.timer_manager.check_timer(timer_id)
    }

    // ===== State Access =====

    /// Get a read-only snapshot of the client state
    /// Note: In the new architecture, scripts should maintain their own state based on events.
    /// This method returns a minimal state snapshot for backward compatibility.
    pub fn client_state(&self) -> ClientStateSnapshot {
        ClientStateSnapshot::new()
    }

    /// Get the timestamp when the current event occurred
    pub fn event_time(&self) -> SystemTime {
        self.event_time
    }
}
