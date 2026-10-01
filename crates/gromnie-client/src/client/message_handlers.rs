//! Message handler trait implementations for S2C messages.
//!
//! This module contains all MessageHandler trait implementations for the Client.
//! Each handler focuses on business logic only - parsing and error handling
//! are centralized in the message_handler module.

use tracing::{error, info, warn};

use crate::client::Client;
use crate::client::ClientEvent;
use crate::client::constants::UI_DELAY_MS;
use crate::client::message_handler::MessageHandler;
use crate::client::messages::{OutgoingMessage, OutgoingMessageContent};
use crate::client::protocol_conversions::ToProtocolEvent;
use crate::client::scene::ClientError;
use asheron_rs::network::RawMessage;
use gromnie_events::{ClientSystemEvent, ProtocolEvent};

/// Handle LoginCreatePlayer messages
impl MessageHandler<asheron_rs::messages::s2c::LoginCreatePlayer> for Client {
    fn handle(&mut self, create_player: asheron_rs::messages::s2c::LoginCreatePlayer) {
        let character_id = create_player.character_id.0;
        info!(target: "net", "Character in world: 0x{:08X}", character_id);

        // Emit protocol event
        self.emit_protocol(create_player.to_protocol_event());

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
    }
}

/// Handle ItemCreateObject messages
impl MessageHandler<asheron_rs::messages::s2c::ItemCreateObject> for Client {
    fn handle(&mut self, create_obj: asheron_rs::messages::s2c::ItemCreateObject) {
        info!(target: "net", "Object created in world: {} (ID: 0x{:08X})",
            create_obj.weenie_description.name, create_obj.object_id.0);

        self.emit_protocol(create_obj.to_protocol_event());
    }
}

/// Handle CommunicationHearSpeech messages
impl MessageHandler<asheron_rs::messages::s2c::CommunicationHearSpeech> for Client {
    fn handle(&mut self, speech: asheron_rs::messages::s2c::CommunicationHearSpeech) {
        self.emit_protocol(speech.to_protocol_event());

        info!(target: "net", "Hear speech received - Type: {}, Text: {}",
            speech.type_ as u32, speech.message);
    }
}

/// Handle CommunicationHearRangedSpeech messages
impl MessageHandler<asheron_rs::messages::s2c::CommunicationHearRangedSpeech> for Client {
    fn handle(&mut self, speech: asheron_rs::messages::s2c::CommunicationHearRangedSpeech) {
        self.emit_protocol(speech.to_protocol_event());

        info!(target: "net", "Hear ranged speech received - Type: {}, Text: {}",
            speech.type_ as u32, speech.message);
    }
}

/// Handle CharacterCharacterError messages
impl MessageHandler<asheron_rs::messages::s2c::CharacterCharacterError> for Client {
    fn handle(&mut self, char_error: asheron_rs::messages::s2c::CharacterCharacterError) {
        let error_code = char_error.reason.clone() as u32;

        error!(target: "net", "Character error received - Code: 0x{:04X} ({})",
            error_code, format!("{}", char_error.reason));

        self.emit_protocol(char_error.to_protocol_event());

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
    }
}

/// Tolerantly read the reason strings from a `LoginAccountBooted` payload.
///
/// Servers send the boot reason as 16-bit-length-prefixed AC strings (padded to
/// a 4-byte boundary). Some servers send only the reason text; the generated
/// `LoginAccountBooted` reader unconditionally reads a second `reason_text`
/// field and fails on that layout, so read whatever strings are present.
pub(crate) fn parse_boot_reason(payload: &[u8]) -> String {
    use asheron_rs::readers::read_string;

    let mut parts = Vec::new();
    let mut cursor = std::io::Cursor::new(payload);
    while let Ok(text) = read_string(&mut cursor) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_string());
        }
    }
    parts.join(" ")
}

/// Handle LoginAccountBooted messages.
///
/// Servers accept any LoginRequest at the transport handshake (they reply with
/// a ConnectRequest regardless of credentials) and reject the credentials
/// afterwards with this message. It must be surfaced as an authentication
/// failure and end the connecting phase — otherwise the client keeps retrying
/// LoginRequest while the server bounces every retry with a fresh
/// ConnectRequest (which re-arms the connecting timeout, pinning the UI on
/// "Connecting…" indefinitely).
impl Client {
    pub(crate) fn handle_login_account_booted(&mut self, message: RawMessage) {
        let reason = if message.data.len() >= 4 {
            parse_boot_reason(&message.data[4..])
        } else {
            String::new()
        };
        let reason = if reason.is_empty() {
            "The server rejected this account.".to_string()
        } else {
            reason
        };

        error!(target: "net", "Account booted — login rejected: {}", reason);

        // Stop the retry loop: this login attempt is over.
        self.transition_to_error(ClientError::Authentication(reason.clone()), false);

        // Surface the failure with the server's reason so UIs return to the
        // server-select screen instead of staying stuck on "Connecting…".
        let _ = self.raw_event_tx.try_send(ClientEvent::System(
            ClientSystemEvent::AuthenticationFailed { reason },
        ));
    }
}

/// Handle LoginLoginCharacterSet messages
impl MessageHandler<asheron_rs::messages::s2c::LoginLoginCharacterSet> for Client {
    fn handle(&mut self, char_list: asheron_rs::messages::s2c::LoginLoginCharacterSet) {
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

        self.emit_protocol(char_list.to_protocol_event());

        // Update progress to 100% after transitioning
        self.emit_progress(ClientSystemEvent::UpdatingProgress { progress: 1.0 });
        info!(target: "net", "Progress: CharacterList received (100%)");

        // Use characters directly from acprotocol message
        let characters = char_list.characters.list.clone();

        // Store the character list for future reference
        self.known_characters = characters.clone();

        // Transition from Patching to CharSelect scene
        self.transition_to_char_select(characters);

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
    }
}

/// Handle DDDInterrogationMessage messages
impl MessageHandler<asheron_rs::messages::s2c::DDDInterrogationMessage> for Client {
    fn handle(&mut self, ddd_msg: asheron_rs::messages::s2c::DDDInterrogationMessage) {
        info!(target: "net", "Received DDD Interrogation - Language: {}, Region: {}, Product: {}",
            ddd_msg.name_rule_language, ddd_msg.servers_region, ddd_msg.product_id);

        self.emit_protocol(ddd_msg.to_protocol_event());

        // Update progress to ReceivedDDD using new scene API
        use crate::client::scene::PatchingProgress as ScenePatchingProgress;
        self.update_patch_progress(ScenePatchingProgress::ReceivedDDD);
        self.emit_progress(ClientSystemEvent::UpdatingProgress { progress: 0.33 });
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
    }
}

/// Handle CharacterCharGenVerificationResponse messages
impl MessageHandler<asheron_rs::messages::s2c::CharacterCharGenVerificationResponse> for Client {
    fn handle(
        &mut self,
        response: asheron_rs::messages::s2c::CharacterCharGenVerificationResponse,
    ) {
        info!(target: "net", "Character creation verification response received");

        self.emit_protocol(response.to_protocol_event());

        // The protocol event carries no character list, so re-publish the known
        // characters as a `LoginCharacterSet` after a short delay. This lets the
        // UI show the character-creation progress before swapping to the list.
        let characters = self.known_characters.clone();
        let account = self.account.name.clone();
        let raw_tx = self.raw_event_tx.clone();
        crate::instant::spawn_detached(async move {
            crate::instant::sleep(std::time::Duration::from_millis(UI_DELAY_MS)).await;
            info!(target: "net", "Sending character list after character creation");
            let event = ProtocolEvent::S2C(gromnie_events::S2CEvent::LoginCharacterSet {
                account,
                characters,
                num_slots: 0,
            });
            if raw_tx.send(ClientEvent::Protocol(event)).await.is_err() {
                error!(target: "net", "Failed to send character list after character creation");
            }
        });
    }
}

/// Handle ItemSetState messages
impl MessageHandler<asheron_rs::messages::s2c::ItemSetState> for Client {
    fn handle(&mut self, state_msg: asheron_rs::messages::s2c::ItemSetState) {
        self.emit_protocol(state_msg.to_protocol_event());
    }
}

/// Handle QualitiesPrivateUpdateInt messages
impl MessageHandler<asheron_rs::messages::s2c::QualitiesPrivateUpdateInt> for Client {
    fn handle(&mut self, quality_msg: asheron_rs::messages::s2c::QualitiesPrivateUpdateInt) {
        self.emit_protocol(quality_msg.to_protocol_event());
    }
}

/// Handle ItemDeleteObject messages
impl MessageHandler<asheron_rs::messages::s2c::ItemDeleteObject> for Client {
    fn handle(&mut self, delete_obj: asheron_rs::messages::s2c::ItemDeleteObject) {
        info!(target: "net", "Object deleted from world: 0x{:08X}", delete_obj.object_id.0);

        self.emit_protocol(delete_obj.to_protocol_event());
    }
}

/// Handle MovementPositionEvent messages (0xF748)
impl MessageHandler<asheron_rs::messages::s2c::MovementPositionEvent> for Client {
    fn handle(&mut self, msg: asheron_rs::messages::s2c::MovementPositionEvent) {
        let object_id = msg.object_id.0;
        let landcell = msg.position.origin.landcell.0;
        info!(target: "net", "Position update: 0x{:08X} at cell 0x{:08X} ({}, {}, {})",
            object_id, landcell,
            msg.position.origin.location.x,
            msg.position.origin.location.y,
            msg.position.origin.location.z);

        self.emit_protocol(msg.to_protocol_event());
    }
}

/// Handle MovementPositionAndMovementEvent messages (0xF619)
impl MessageHandler<asheron_rs::messages::s2c::MovementPositionAndMovementEvent> for Client {
    fn handle(&mut self, msg: asheron_rs::messages::s2c::MovementPositionAndMovementEvent) {
        let object_id = msg.object_id.0;
        let landcell = msg.position.origin.landcell.0;
        info!(target: "net", "Position+movement update: 0x{:08X} at cell 0x{:08X} ({}, {}, {})",
            object_id, landcell,
            msg.position.origin.location.x,
            msg.position.origin.location.y,
            msg.position.origin.location.z);

        self.emit_protocol(msg.to_protocol_event());
    }
}

/// Handle MovementSetObjectMovement messages (0xF74C)
impl MessageHandler<asheron_rs::messages::s2c::MovementSetObjectMovement> for Client {
    fn handle(&mut self, msg: asheron_rs::messages::s2c::MovementSetObjectMovement) {
        info!(target: "net", "Object movement update: 0x{:08X} (seq {})",
            msg.object_id.0, msg.object_instance_sequence);

        self.emit_protocol(msg.to_protocol_event());
    }
}

/// Handle EffectsPlayerTeleport messages (0xF751)
impl MessageHandler<asheron_rs::messages::s2c::EffectsPlayerTeleport> for Client {
    fn handle(&mut self, msg: asheron_rs::messages::s2c::EffectsPlayerTeleport) {
        info!(target: "net", "Player teleport effect (seq {})", msg.object_teleport_sequence);

        self.emit_protocol(msg.to_protocol_event());
    }
}
