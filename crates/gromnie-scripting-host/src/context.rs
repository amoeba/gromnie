use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use super::timer::TimerId;
use asheron_rs::message::GameActionMessage;
use gromnie_client::client::Client;
use gromnie_events::SimpleClientAction;

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
    /// Channel for sending client actions
    action_tx: UnboundedSender<SimpleClientAction>,
    /// Channel for sending GameActionMessage directly to the client
    game_action_tx: UnboundedSender<GameActionMessage>,
    /// Shared timer manager
    timer_manager: Arc<super::timer::TimerManager>,
    /// Timestamp when the current event occurred
    event_time: SystemTime,
}

impl ScriptContext {
    /// Create a new script context
    pub(crate) async fn new(
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
        timer_manager: Arc<super::timer::TimerManager>,
        event_time: SystemTime,
    ) -> Self {
        let game_action_tx = client.read().await.game_action_tx.clone();
        Self {
            client,
            action_tx,
            game_action_tx,
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
        let _ = self.action_tx.send(SimpleClientAction::SendChatSay {
            message: message.into(),
        });
    }

    /// Send a direct message to a specific player
    pub fn send_tell(&self, recipient: impl Into<String>, message: impl Into<String>) {
        let _ = self.action_tx.send(SimpleClientAction::SendChatTell {
            recipient_name: recipient.into(),
            message: message.into(),
        });
    }

    // ===== Trading =====

    pub fn open_trade(&self, partner_id: u32) {
        use asheron_rs::gameactions::TradeOpenTradeNegotiations;
        use asheron_rs::types::ObjectId;
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeOpenTradeNegotiations(
                TradeOpenTradeNegotiations {
                    object_id: ObjectId(partner_id),
                },
            ));
    }

    pub fn add_to_trade(&self, item_id: u32, slot: u32) {
        use asheron_rs::gameactions::TradeAddToTrade;
        use asheron_rs::types::ObjectId;
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeAddToTrade(TradeAddToTrade {
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
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeAcceptTrade(TradeAcceptTrade {
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
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeDeclineTrade(TradeDeclineTrade {}));
    }

    pub fn reset_trade(&self) {
        use asheron_rs::gameactions::TradeResetTrade;
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeResetTrade(TradeResetTrade {}));
    }

    pub fn close_trade(&self) {
        use asheron_rs::gameactions::TradeCloseTradeNegotiations;
        let _ = self
            .game_action_tx
            .send(GameActionMessage::TradeCloseTradeNegotiations(
                TradeCloseTradeNegotiations {},
            ));
    }

    // ===== Spell Casting =====

    pub fn cast_targeted_spell(&self, target_id: u32, spell_id: u32) {
        use asheron_rs::gameactions::MagicCastTargetedSpell;
        use asheron_rs::types::{LayeredSpellId, ObjectId, SpellId};
        let _ = self
            .game_action_tx
            .send(GameActionMessage::MagicCastTargetedSpell(
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
        let _ = self
            .game_action_tx
            .send(GameActionMessage::MagicCastUntargetedSpell(
                MagicCastUntargetedSpell {
                    spell_id: LayeredSpellId {
                        id: SpellId(spell_id as u16),
                        layer: 0,
                    },
                },
            ));
    }

    /// Send a client action
    pub fn send_action(&self, action: SimpleClientAction) {
        let _ = self.action_tx.send(action);
    }

    /// Send a raw message (advanced usage) - deprecated, use send_action instead
    pub fn send_message_deprecated(&self, _message: &str) {
        // This method is no longer supported with SimpleClientAction
        // Use send_action() with the appropriate variant instead
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
