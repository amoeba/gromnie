//! Message handler trait implementations for S2C messages.
//!
//! This module contains all MessageHandler trait implementations for the Client.
//! Each handler focuses on business logic only - parsing and error handling
//! are centralized in the message_handler module.

use tracing::{error, info, warn};

use crate::client::Client;
use crate::client::constants::UI_DELAY_MS;
use crate::client::message_handler::MessageHandler;
use crate::client::messages::{OutgoingMessage, OutgoingMessageContent};
use crate::client::protocol_conversions::ToProtocolEvent;
use crate::client::scene::ClientError;
use crate::client::{ClientEvent, GameEvent};
use gromnie_events::ProtocolEvent;

/// Handle LoginCreatePlayer messages
impl MessageHandler<acprotocol::messages::s2c::LoginCreatePlayer> for Client {
    fn handle(
        &mut self,
        create_player: acprotocol::messages::s2c::LoginCreatePlayer,
    ) -> Option<GameEvent> {
        let character_id = create_player.character_id.0;
        info!(target: "net", "Character in world: 0x{:08X}", character_id);

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(create_player.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        // Check if we're in the process of entering the world
        if let Some(entering) = self
            .scene
            .as_character_select()
            .and_then(|scene| scene.entering_world.as_ref().cloned())
        {
            // Send the login complete notification
            // This will also handle the transition to InWorld and emit LoginSucceeded event
            if !entering.login_complete {
                self.send_login_complete_notification();
            } else {
                warn!(target: "net", "LoginCreatePlayer: login_complete already marked, skipping send");
            }

            info!(target: "net", "Character successfully entered world: {} (ID: 0x{:08X})",
                  entering.character_name, character_id);
        } else {
            warn!(target: "net", "LoginCreatePlayer received but not in CharacterSelect with entering_world state");
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

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(create_obj.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        Some(GameEvent::ItemCreateObject {
            object_id,
            name: object_name,
            item_type: format!("{:?}", create_obj.weenie_description.type_),
            container_id: create_obj.weenie_description.container_id.map(|id| id.0),
            burden: create_obj.weenie_description.burden.unwrap_or(0) as u32,
            value: create_obj.weenie_description.value.unwrap_or(0),
            items_capacity: create_obj
                .weenie_description
                .items_capacity
                .map(|c| c as u32),
            container_capacity: create_obj
                .weenie_description
                .container_capacity
                .map(|c| c as u32),
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
        let message_type = speech.type_.clone() as u32;

        info!(target: "net", "Hear speech received - Type: {}, Text: {}", message_type, chat_text);

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(speech.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

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
        let message_type = speech.type_.clone() as u32;

        info!(target: "net", "Hear ranged speech received - Type: {}, Text: {}", message_type, chat_text);

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(speech.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

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

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(char_error.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        // ServerCrash (0x0004) means the server is going down - trigger reconnection
        if error_code == 0x0004 {
            warn!(target: "net", "ServerCrash received - entering Disconnected state for reconnection");
            self.enter_disconnected();
        } else {
            // Other character errors are fatal - transition to Error scene
            self.transition_to_error(
                ClientError::CharacterError(char_error.reason.clone()),
                true, // Can retry from character error
            );
        }

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

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(char_list.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        // Use characters directly from acprotocol message
        let characters = char_list.characters.list.clone();

        // Store the character list for future reference
        self.known_characters = characters.clone();

        // Transition from Patching to CharSelect scene
        self.transition_to_char_select(characters.clone());

        // Update progress to 100% after transitioning
        let progress_event = GameEvent::UpdatingSetProgress { progress: 1.0 };
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Game(progress_event));
        info!(target: "net", "Progress: CharacterList received (100%)");

        // Clear cached DDD response since we successfully received character list
        self.ddd_response = None;

        // Reset reconnect attempt counter on successful connection
        if self.reconnect_attempt_count > 0 {
            info!(target: "net", "Connection successful - resetting reconnect attempt counter from {} to 0",
                self.reconnect_attempt_count);
            self.reconnect_attempt_count = 0;
        }

        info!(target: "net", "Scene transition: Connecting (Patching) -> CharacterSelect");

        // Check if auto-login is configured
        if let Some(ref char_name) = self.character {
            // Find the character in the list
            let found_char = self
                .known_characters
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(char_name) && c.seconds_greyed_out == 0);

            if let Some(character) = found_char {
                info!(target: "net", "Auto-login enabled, queuing login for character: {} (ID: {})", character.name, character.character_id.0);

                // Store the pending auto-login action to be processed in the main loop
                self.pending_auto_login =
                    Some(gromnie_events::SimpleClientAction::LoginCharacter {
                        character_id: character.character_id.0,
                        character_name: character.name.clone(),
                        account: char_list.account.clone(),
                    });
            } else {
                let available_names: Vec<&str> = self
                    .known_characters
                    .iter()
                    .filter(|c| c.seconds_greyed_out == 0)
                    .map(|c| c.name.as_str())
                    .collect();

                error!(target: "net", "Auto-login character '{}' not found in character list. Available characters: [{}]",
                    char_name, available_names.join(", "));
            }
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
        info!(target: "net", "Received DDD Interrogation - Language: {}, Region: {}, Product: {}",
            ddd_msg.name_rule_language, ddd_msg.servers_region, ddd_msg.product_id);

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(ddd_msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        // Update progress to ReceivedDDD using new scene API
        use crate::client::scene::PatchingProgress as ScenePatchingProgress;
        self.update_patch_progress(ScenePatchingProgress::ReceivedDDD);
        let game_event = GameEvent::UpdatingSetProgress { progress: 0.33 };
        let _ = self.raw_event_tx.try_send(ClientEvent::Game(game_event));
        info!(target: "net", "Progress: DDDInterrogation received (33%)");

        // Send static DDD response indicating client is up-to-date
        info!(target: "net", "Sending DDD Interrogation Response (up-to-date, no patches needed)");

        let response_content = OutgoingMessageContent::GameAction(
            crate::client::constants::DDD_RESPONSE_UP_TO_DATE.to_vec(),
        );
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
        response: acprotocol::messages::s2c::CharacterCharGenVerificationResponse,
    ) -> Option<GameEvent> {
        info!(target: "net", "Character creation verification response received");

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(response.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

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

/// Handle ItemSetState messages
impl MessageHandler<acprotocol::messages::s2c::ItemSetState> for Client {
    fn handle(&mut self, state_msg: acprotocol::messages::s2c::ItemSetState) -> Option<GameEvent> {
        let object_id = state_msg.object_id.0;
        let new_state = format!("{:?}", state_msg.new_state);

        info!(target: "net", "ItemSetState: Object {} state changed to {}", object_id, new_state);

        // For now, we'll just emit as a generic state property update
        Some(GameEvent::ItemSetState {
            object_id,
            property_name: "State".to_string(),
            value: 0, // State is a bitfield, not a simple int
        })
    }
}

/// Handle QualitiesPrivateUpdateInt messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateInt> for Client {
    fn handle(
        &mut self,
        quality_msg: acprotocol::messages::s2c::QualitiesPrivateUpdateInt,
    ) -> Option<GameEvent> {
        let property_name = format!("{:?}", quality_msg.key);
        let value = quality_msg.value;

        info!(target: "net", "QualitiesPrivateUpdateInt: Property {} = {}", property_name, value);

        // This is a global quality update, not tied to a specific object
        // For now we'll emit with object_id 0 (or handle this differently)
        // In reality, this might update the player or a specific object
        Some(GameEvent::QualitiesPrivateUpdateInt {
            object_id: 0, // TODO: determine which object this applies to
            property_name,
            value,
        })
    }
}

/// Handle ItemDeleteObject messages
impl MessageHandler<acprotocol::messages::s2c::ItemDeleteObject> for Client {
    fn handle(
        &mut self,
        delete_obj: acprotocol::messages::s2c::ItemDeleteObject,
    ) -> Option<GameEvent> {
        let object_id = delete_obj.object_id.0;

        info!(target: "net", "Object deleted from world: 0x{:08X}", object_id);

        // Emit protocol event
        let protocol_event = ProtocolEvent::S2C(delete_obj.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));

        Some(GameEvent::ItemDeleteObject { object_id })
    }
}

// ============================================================================
// Quality/Property Update Handlers
// ============================================================================

/// Handle QualitiesUpdateInt messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateInt> for Client {
    fn handle(&mut self, msg: acprotocol::messages::s2c::QualitiesUpdateInt) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateInt: Object {} property {:?} = {}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateInt64 messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateInt64> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesUpdateInt64,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateInt64: Object {} property {:?} = {}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateInt64 messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateInt64> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateInt64,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateInt64: property {:?} = {}",
              msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateBool messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateBool> for Client {
    fn handle(&mut self, msg: acprotocol::messages::s2c::QualitiesUpdateBool) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateBool: Object {} property {:?} = {}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateBool messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateBool> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateBool,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateBool: property {:?} = {}",
              msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateFloat messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateFloat> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesUpdateFloat,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateFloat: Object {} property {:?} = {}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateFloat messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateFloat> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateFloat,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateFloat: property {:?} = {}",
              msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateString messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateString> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesUpdateString,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateString: Object {} property {:?} = {}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateString messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateString> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateString,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateString: property {:?} = {}",
              msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateDataId messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateDataId> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesUpdateDataId,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateDataId: Object {} property {:?} = 0x{:08X}",
              msg.object_id.0, msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateDataId messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateDataId> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateDataId,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateDataId: property {:?} = 0x{:08X}",
              msg.key, msg.value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesUpdateInstanceId messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesUpdateInstanceId> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesUpdateInstanceId,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesUpdateInstanceId: Object {} property {:?} = 0x{:08X}",
              msg.object_id.0, msg.key, msg.value.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle QualitiesPrivateUpdateInstanceId messages
impl MessageHandler<acprotocol::messages::s2c::QualitiesPrivateUpdateInstanceId> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::QualitiesPrivateUpdateInstanceId,
    ) -> Option<GameEvent> {
        info!(target: "net", "QualitiesPrivateUpdateInstanceId: property {:?} = 0x{:08X}",
              msg.key, msg.value.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// Note: Position updates are complex and need serialization, skipping for now but handler is registered

// ============================================================================
// Communication Message Handlers
// ============================================================================

/// Handle CommunicationHearEmote messages
impl MessageHandler<acprotocol::messages::s2c::CommunicationHearEmote> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::CommunicationHearEmote,
    ) -> Option<GameEvent> {
        info!(target: "net", "HearEmote: {} {}", msg.sender_name, msg.text);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle CommunicationHearSoulEmote messages
impl MessageHandler<acprotocol::messages::s2c::CommunicationHearSoulEmote> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::CommunicationHearSoulEmote,
    ) -> Option<GameEvent> {
        info!(target: "net", "HearSoulEmote: {} {}", msg.sender_name, msg.text);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// ============================================================================
// Item/Inventory Message Handlers
// ============================================================================

/// Handle ItemUpdateStackSize messages
impl MessageHandler<acprotocol::messages::s2c::ItemUpdateStackSize> for Client {
    fn handle(&mut self, msg: acprotocol::messages::s2c::ItemUpdateStackSize) -> Option<GameEvent> {
        info!(target: "net", "ItemUpdateStackSize: Object {} new value = {}",
              msg.object_id.0, msg.new_value);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle ItemServerSaysRemove messages
impl MessageHandler<acprotocol::messages::s2c::ItemServerSaysRemove> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::ItemServerSaysRemove,
    ) -> Option<GameEvent> {
        info!(target: "net", "ItemServerSaysRemove: Object {}", msg.object_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle InventoryPickupEvent messages
impl MessageHandler<acprotocol::messages::s2c::InventoryPickupEvent> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::InventoryPickupEvent,
    ) -> Option<GameEvent> {
        info!(target: "net", "InventoryPickupEvent: Object {}", msg.object_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// ============================================================================
// Effects Message Handlers
// ============================================================================

/// Handle EffectsSoundEvent messages
impl MessageHandler<acprotocol::messages::s2c::EffectsSoundEvent> for Client {
    fn handle(&mut self, msg: acprotocol::messages::s2c::EffectsSoundEvent) -> Option<GameEvent> {
        info!(target: "net", "EffectsSoundEvent: Object {} sound type 0x{:08X}",
              msg.object_id.0, msg.sound_type.clone() as u32);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// ============================================================================
// Movement Message Handlers
// ============================================================================

/// Handle MovementPositionAndMovementEvent messages
impl MessageHandler<acprotocol::messages::s2c::MovementPositionAndMovementEvent> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::MovementPositionAndMovementEvent,
    ) -> Option<GameEvent> {
        info!(target: "net", "MovementPositionAndMovement: Object 0x{:08X}", msg.object_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle MovementPositionEvent messages
impl MessageHandler<acprotocol::messages::s2c::MovementPositionEvent> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::MovementPositionEvent,
    ) -> Option<GameEvent> {
        info!(target: "net", "MovementPosition: Object 0x{:08X}", msg.object_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle MovementSetObjectMovement messages
impl MessageHandler<acprotocol::messages::s2c::MovementSetObjectMovement> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::MovementSetObjectMovement,
    ) -> Option<GameEvent> {
        info!(target: "net", "MovementSetObjectMovement: Object 0x{:08X}", msg.object_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

/// Handle MovementVectorUpdate messages
impl MessageHandler<acprotocol::messages::s2c::MovementVectorUpdate> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::MovementVectorUpdate,
    ) -> Option<GameEvent> {
        info!(target: "net", "MovementVectorUpdate: Object 0x{:08X} velocity ({}, {}, {})",
              msg.object_id.0, msg.velocity.x, msg.velocity.y, msg.velocity.z);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// ============================================================================
// Combat Message Handlers
// ============================================================================

/// Handle CombatHandlePlayerDeathEvent messages
impl MessageHandler<acprotocol::messages::s2c::CombatHandlePlayerDeathEvent> for Client {
    fn handle(
        &mut self,
        msg: acprotocol::messages::s2c::CombatHandlePlayerDeathEvent,
    ) -> Option<GameEvent> {
        info!(target: "net", "CombatHandlePlayerDeath: {} killed by 0x{:08X}",
              msg.killed_id.0, msg.killer_id.0);

        let protocol_event = ProtocolEvent::S2C(msg.to_protocol_event());
        let _ = self
            .raw_event_tx
            .try_send(ClientEvent::Protocol(protocol_event));
        None
    }
}

// ============================================================================
// Magic & Items Message Handlers
// ============================================================================
// These handlers cover enchantments, appraisal, equipment, containers,
// and item queries as part of the protocol coverage.

// Magic & Enchantment Handlers
// Handlers for MagicUpdateEnchantment, MagicRemoveEnchantment, etc.
// (Awaiting acprotocol message type definitions)

// Item Appraisal Handlers
// Handlers for ItemAppriseInfo, ItemAppriseInfoDone
// (Awaiting acprotocol message type definitions)

// Equipment/Wear Handlers
// Handlers for ItemWearOutfit, ItemUnwearOutfit
// (Awaiting acprotocol message type definitions)

// Container/Inventory Handlers
// Handlers for ItemContainersViewData, ItemContainerIdUpdate, ItemMoveItem*
// (Awaiting acprotocol message type definitions)

// Item Query Handlers
// Handlers for ItemQueryItemManaResponse, ItemGetInscriptionResponse, etc.
// (Awaiting acprotocol message type definitions)
