use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use super::timer::TimerId;
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
    /// Reference to the timer manager (will be updated by ScriptRunner)
    timer_manager: *mut super::timer::TimerManager,
    /// Timestamp when the current event occurred
    event_time: SystemTime,
}

impl ScriptContext {
    /// Create a new script context
    ///
    /// # Safety
    /// The timer_manager pointer must remain valid for the lifetime of this context
    pub(crate) unsafe fn new(
        client: Arc<RwLock<Client>>,
        action_tx: UnboundedSender<SimpleClientAction>,
        timer_manager: *mut super::timer::TimerManager,
        event_time: SystemTime,
    ) -> Self {
        Self {
            client,
            action_tx,
            timer_manager,
            event_time,
        }
    }

    /// Get current client state (clones session and scene from the client)
    pub fn client(&self) -> ClientState {
        // This blocks but should be very fast since we're just cloning
        let client_guard = self.client.blocking_read();
        ClientState {
            session: client_guard.session.clone(),
            scene: client_guard.scene.clone(),
        }
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
    pub fn schedule_timer(&mut self, delay_secs: u64, name: impl Into<String>) -> TimerId {
        unsafe {
            (*self.timer_manager).schedule_timer(Duration::from_secs(delay_secs), name.into())
        }
    }

    /// Schedule a recurring timer that fires repeatedly at an interval
    pub fn schedule_recurring(&mut self, interval_secs: u64, name: impl Into<String>) -> TimerId {
        unsafe {
            (*self.timer_manager)
                .schedule_recurring(Duration::from_secs(interval_secs), name.into())
        }
    }

    /// Cancel a timer
    pub fn cancel_timer(&mut self, timer_id: TimerId) -> bool {
        unsafe { (*self.timer_manager).cancel_timer(timer_id) }
    }

    /// Check if a timer has fired (consumes the fired state)
    pub fn check_timer(&mut self, timer_id: TimerId) -> bool {
        unsafe { (*self.timer_manager).check_timer(timer_id) }
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
