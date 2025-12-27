use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::client::events::{CharacterInfo, ClientAction, GameEvent};
use crate::client::OutgoingMessageContent;
use crate::runner::CharacterBuilder;

/// Trait for consuming game events - allows different implementations for CLI vs TUI
pub trait EventConsumer: Send + 'static {
    /// Handle a game event
    fn handle_event(&mut self, event: GameEvent);
}

/// Event consumer that logs events to the console (for CLI version)
pub struct LoggingConsumer {
    action_tx: UnboundedSender<ClientAction>,
    character_created: Arc<AtomicBool>,
}

impl LoggingConsumer {
    pub fn new(action_tx: UnboundedSender<ClientAction>) -> Self {
        Self {
            action_tx,
            character_created: Arc::new(AtomicBool::new(false)),
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
                );
            }
            GameEvent::DDDInterrogation { language, region } => {
                info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                info!(target: "events", "=== LOGIN SUCCEEDED === Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);

                // Send a chat message after successful login
                info!(target: "events", "Sending chat message...");
                if let Err(e) = self.action_tx.send(ClientAction::SendChatMessage {
                    message: "Hello from gromnie!".to_string(),
                }) {
                    error!(target: "events", "Failed to send chat message: {}", e);
                }
            }
            GameEvent::LoginFailed { reason } => {
                error!(target: "events", "=== LOGIN FAILED ===");
                error!(target: "events", "Reason: {}", reason);
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
            // Ignore progress events in the CLI version
            GameEvent::ConnectingSetProgress { .. }
            | GameEvent::UpdatingSetProgress { .. }
            | GameEvent::FakeProgressComplete
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
                );
            }
            GameEvent::DDDInterrogation { language, region } => {
                info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
            }
            GameEvent::LoginSucceeded {
                character_id,
                character_name,
            } => {
                info!(target: "events", "=== LOGIN SUCCEEDED === Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);
            }
            GameEvent::LoginFailed { reason } => {
                error!(target: "events", "=== LOGIN FAILED ===");
                error!(target: "events", "Reason: {}", reason);
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
            // Progress events are handled by TUI directly
            GameEvent::ConnectingSetProgress { .. }
            | GameEvent::UpdatingSetProgress { .. }
            | GameEvent::FakeProgressComplete
            | GameEvent::ConnectingStart
            | GameEvent::ConnectingDone
            | GameEvent::UpdatingStart
            | GameEvent::UpdatingDone => {}
        }
    }
}

/// Shared logic for handling character list received event
fn handle_character_list(
    account: &str,
    characters: &[CharacterInfo],
    num_slots: u32,
    action_tx: &UnboundedSender<ClientAction>,
    character_created: &Arc<AtomicBool>,
) {
    info!(target: "events", "=== Character List Event ===");
    info!(target: "events", "Account: {}", account);
    info!(target: "events", "Slots: {}", num_slots);
    info!(target: "events", "Number of characters: {}", characters.len());

    // Print character names
    for char_info in characters {
        if char_info.delete_pending {
            info!(target: "events", "  - {} (ID: {}) [PENDING DELETION]", char_info.name, char_info.id);
        } else {
            info!(target: "events", "  - {} (ID: {})", char_info.name, char_info.id);
        }
    }

    // If we don't have any characters, create one
    if characters.is_empty() && !character_created.load(Ordering::SeqCst) {
        info!(target: "events", "No characters found - creating a new character...");

        // Mark that we're creating a character
        character_created.store(true, Ordering::SeqCst);

        // Create character using builder
        let char_gen_result = CharacterBuilder::new_test_character().build();
        let char_name = char_gen_result.name.clone();

        info!(target: "events", "Creating character: {}", char_name);

        let msg =
            OutgoingMessageContent::CharacterCreationAce(account.to_string(), char_gen_result);
        if let Err(e) = action_tx.send(ClientAction::SendMessage(Box::new(msg))) {
            error!(target: "events", "Failed to send character creation action: {}", e);
        } else {
            info!(target: "events", "Character creation action sent - waiting for response...");
        }
    }
    // If we have characters, log in as the first one
    else if !characters.is_empty() {
        info!(target: "events", "Found existing character(s):");
        for char_info in characters {
            info!(target: "events", "  Character: {} (ID: {})", char_info.name, char_info.id);
        }

        // Log in as the first character
        let first_char = &characters[0];
        info!(target: "events", "Attempting to log in as: {} (ID: {})", first_char.name, first_char.id);

        // Send action to login
        if let Err(e) = action_tx.send(ClientAction::LoginCharacter {
            character_id: first_char.id,
            character_name: first_char.name.clone(),
            account: account.to_string(),
        }) {
            error!(target: "events", "Failed to send login action: {}", e);
        } else {
            // Mark that we've handled character login
            character_created.store(true, Ordering::SeqCst);
        }
    }
}
