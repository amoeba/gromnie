//! Message handler trait implementations for S2C messages.
//!
//! This module contains all MessageHandler trait implementations for the Client.
//! Each handler focuses on business logic only - parsing and error handling
//! are centralized in the message_handler module.

use tracing::{error, info};

use crate::client::client::CharacterLoginState;
use crate::client::constants::UI_DELAY_MS;
use crate::client::message_handler::MessageHandler;
use crate::client::messages::{OutgoingMessage, OutgoingMessageContent};
use crate::client::{CharacterInfo, ClientEvent, GameEvent};
use crate::client::{Client, ClientState, PatchingProgress};

/// Handle LoginCreatePlayer messages
impl MessageHandler<acprotocol::messages::s2c::LoginCreatePlayer> for Client {
    fn handle(
        &mut self,
        create_player: acprotocol::messages::s2c::LoginCreatePlayer,
    ) -> Option<GameEvent> {
        let character_id = create_player.character_id.0;
        info!(target: "net", "Character in world: 0x{:08X}", character_id);

        if self.character_login_state == CharacterLoginState::LoadingWorld {
            self.send_login_complete_notification();
        }

        Some(GameEvent::CreatePlayer { character_id })
    }
}
/// Handle ItemCreateObject messages
impl MessageHandler<acprotocol::messages::s2c::ItemCreateObject> for Client {
    fn handle(
        &mut self,
        create_obj: acprotocol::messages::s2c::ItemCreateObject,
    ) -> Option<GameEvent> {
        let object_id = create_obj.object_id.0;
        let object_name = create_obj.weenie_description.name.clone();

        info!(target: "net", "Object created in world: {} (ID: 0x{:08X})", object_name, object_id);

        Some(GameEvent::CreateObject {
            object_id,
            object_name,
        })
    }
}

/// Handle CommunicationHearSpeech messages
impl MessageHandler<acprotocol::messages::s2c::CommunicationHearSpeech> for Client {
    fn handle(
        &mut self,
        speech: acprotocol::messages::s2c::CommunicationHearSpeech,
    ) -> Option<GameEvent> {
        let chat_text = format!("{} says, \"{}\"", speech.sender_name, speech.message);
        let message_type = speech.type_ as u32;

        info!(target: "net", "Hear speech received - Type: {}, Text: {}", message_type, chat_text);

        Some(GameEvent::ChatMessageReceived {
            message: chat_text,
            message_type,
        })
    }
}

/// Handle CommunicationHearRangedSpeech messages
impl MessageHandler<acprotocol::messages::s2c::CommunicationHearRangedSpeech> for Client {
    fn handle(
        &mut self,
        speech: acprotocol::messages::s2c::CommunicationHearRangedSpeech,
    ) -> Option<GameEvent> {
        let chat_text = format!("{} says, \"{}\"", speech.sender_name, speech.message);
        let message_type = speech.type_ as u32;

        info!(target: "net", "Hear ranged speech received - Type: {}, Text: {}", message_type, chat_text);

        Some(GameEvent::ChatMessageReceived {
            message: chat_text,
            message_type,
        })
    }
}

/// Handle CharacterCharacterError messages
impl MessageHandler<acprotocol::messages::s2c::CharacterCharacterError> for Client {
    fn handle(
        &mut self,
        char_error: acprotocol::messages::s2c::CharacterCharacterError,
    ) -> Option<GameEvent> {
        let error_code = char_error.reason.clone() as u32;
        let error_message = format!("{}", char_error.reason);

        error!(target: "net", "Character error received - Code: 0x{:04X} ({})", error_code, error_message);

        // Transition to CharacterError state (fatal error)
        self.state = ClientState::CharacterError {
            reason: char_error.reason.clone(),
        };

        Some(GameEvent::CharacterError {
            error_code,
            error_message,
        })
    }
}

/// Handle LoginLoginCharacterSet messages
impl MessageHandler<acprotocol::messages::s2c::LoginLoginCharacterSet> for Client {
    fn handle(
        &mut self,
        char_list: acprotocol::messages::s2c::LoginLoginCharacterSet,
    ) -> Option<GameEvent> {
        // Format character list for logging
        let chars = char_list
            .characters
            .list
            .iter()
            .map(|c| {
                if c.seconds_greyed_out > 0 {
                    format!(
                        "{} (ID: {:?}) [PENDING DELETION in {} seconds]",
                        c.name, c.character_id, c.seconds_greyed_out
                    )
                } else {
                    format!("{} (ID: {:?})", c.name, c.character_id)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        info!(target: "net", "CharacterList -- Account: {}, Slots: {}, Characters: [{}]",
            char_list.account, char_list.num_allowed_characters, chars);

        // Create character info list
        let characters: Vec<CharacterInfo> = char_list
            .characters
            .list
            .iter()
            .map(|c| CharacterInfo {
                name: c.name.clone(),
                id: c.character_id.0,
                delete_pending: c.seconds_greyed_out > 0,
            })
            .collect();

        // Store the character list for future reference
        self.known_characters = characters.clone();

        // Transition from Patching to CharSelect state
        if matches!(self.state, ClientState::Patching { .. }) {
            // Update progress to 100% before transitioning
            let progress_event = GameEvent::UpdatingSetProgress { progress: 1.0 };
            let _ = self
                .raw_event_tx
                .try_send(ClientEvent::Game(progress_event));
            info!(target: "net", "Progress: CharacterList received (100%)");

            let _old_state = std::mem::replace(&mut self.state, ClientState::CharSelect);

            // Clear cached DDD response since we successfully received character list
            self.ddd_response = None;
            info!(target: "net", "State transition: Patching -> CharSelect");
        }

        // Delay sending the CharacterListReceived event (to make UI progress visible)
        let game_event = GameEvent::CharacterListReceived {
            account: char_list.account.clone(),
            characters,
            num_slots: char_list.num_allowed_characters,
        };
        let raw_tx = self.raw_event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(UI_DELAY_MS)).await;
            info!(target: "net", "Sending CharacterListReceived event after delay");
            if raw_tx.send(ClientEvent::Game(game_event)).await.is_err() {
                error!(target: "net", "Failed to send CharacterListReceived event");
            } else {
                info!(target: "net", "CharacterListReceived event sent successfully");
            }
        });
        info!(target: "net", "CharacterListReceived event scheduled with {}ms delay", UI_DELAY_MS);

        // Return None since event is sent asynchronously
        None
    }
}

/// Handle DDDInterrogationMessage messages
impl MessageHandler<acprotocol::messages::s2c::DDDInterrogationMessage> for Client {
    fn handle(
        &mut self,
        ddd_msg: acprotocol::messages::s2c::DDDInterrogationMessage,
    ) -> Option<GameEvent> {
        use acprotocol::messages::c2s::DDDInterrogationResponseMessage;
        use acprotocol::types::PackableList;

        info!(target: "net", "Received DDD Interrogation - Language: {}, Region: {}, Product: {}",
            ddd_msg.name_rule_language, ddd_msg.servers_region, ddd_msg.product_id);

        // Update progress to DDDInterrogationReceived (33%)
        if let ClientState::Patching {
            started_at: _,
            last_retry_at: _,
            progress,
        } = &mut self.state
        {
            *progress = PatchingProgress::DDDInterrogationReceived;
            let game_event = GameEvent::UpdatingSetProgress { progress: 0.33 };
            let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));
            info!(target: "net", "Progress: DDDInterrogation received (33%)");
        }

        // Prepare response with language 1 and the file list from the pcap
        let files = vec![4294967296, -8899172235240, 4294967297];
        let response = DDDInterrogationResponseMessage {
            language: 1,
            files: PackableList {
                count: files.len() as u32,
                list: files,
            },
        };

        // Cache the response for retries
        let response_content = OutgoingMessageContent::DDDInterrogationResponse(response);
        self.ddd_response = Some(response_content.clone());

        // Queue the response with delay (to make UI progress visible)
        self.outgoing_message_queue
            .push_back(OutgoingMessage::new(response_content).with_delay_ms(UI_DELAY_MS));
        info!(target: "net", "DDD response cached and queued for sending with {}ms delay", UI_DELAY_MS);

        None
    }
}

/// Handle CharacterCharGenVerificationResponse messages
impl MessageHandler<acprotocol::messages::s2c::CharacterCharGenVerificationResponse> for Client {
    fn handle(
        &mut self,
        _response: acprotocol::messages::s2c::CharacterCharGenVerificationResponse,
    ) -> Option<GameEvent> {
        info!(target: "net", "Character creation verification response received");

        // Delay emitting CharacterListReceived event
        let game_event = GameEvent::CharacterListReceived {
            account: String::new(),
            characters: self.known_characters.clone(),
            num_slots: 0,
        };
        let raw_tx = self.raw_event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(UI_DELAY_MS)).await;
            info!(target: "net", "Sending CharacterListReceived event after character creation");
            if raw_tx.send(ClientEvent::Game(game_event)).await.is_err() {
                error!(target: "net", "Failed to send CharacterListReceived event");
            } else {
                info!(target: "net", "CharacterListReceived event sent successfully");
            }
        });

        None
    }
}
