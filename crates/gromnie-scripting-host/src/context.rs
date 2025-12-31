use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

use super::timer::TimerId;
use gromnie_client::client::OutgoingMessageContent;
use gromnie_client::client::events::ClientAction;

/// Snapshot of client state at the time of an event
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
    /// Channel for sending client actions
    action_tx: UnboundedSender<ClientAction>,
    /// Reference to the timer manager (will be updated by ScriptRunner)
    timer_manager: *mut super::timer::TimerManager,
    /// Timestamp when the current event occurred
    event_time: Instant,
}

impl ScriptContext {
    /// Create a new script context
    ///
    /// # Safety
    /// The timer_manager pointer must remain valid for the lifetime of this context
    pub(crate) unsafe fn new(
        action_tx: UnboundedSender<ClientAction>,
        timer_manager: *mut super::timer::TimerManager,
        event_time: Instant,
    ) -> Self {
        Self {
            action_tx,
            timer_manager,
            event_time,
        }
    }

    // ===== Action Methods =====

    /// Send a chat message
    pub fn send_chat(&self, message: impl Into<String>) {
        let _ = self.action_tx.send(ClientAction::SendChatMessage {
            message: message.into(),
        });
    }

    /// Send a client action
    pub fn send_action(&self, action: ClientAction) {
        let _ = self.action_tx.send(action);
    }

    /// Send a raw message (advanced usage)
    pub fn send_message(&self, message: OutgoingMessageContent) {
        let _ = self
            .action_tx
            .send(ClientAction::SendMessage(Box::new(message)));
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

    /// Get a read-only snapshot of the client state (removed - scripts receive state via events)

    /// Get the timestamp when the current event occurred
    pub fn event_time(&self) -> Instant {
        self.event_time
    }
}
