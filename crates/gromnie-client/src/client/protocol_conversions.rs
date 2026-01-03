//! Conversion implementations from acprotocol types to ProtocolEvent types
//!
//! This module contains helper functions that convert acprotocol message
//! types into our strongly-typed ProtocolEvent wrapper types. These are here
//! (instead of gromnie-events) to avoid circular dependencies, since only
//! gromnie-client depends on acprotocol.

use gromnie_events::{GameEventMsg, S2CEvent};

/// Helper trait for converting acprotocol S2C message types to ProtocolEvent-compatible types
pub trait ToProtocolEvent {
    fn to_protocol_event(&self) -> S2CEvent;
}

impl ToProtocolEvent for acprotocol::messages::s2c::LoginCreatePlayer {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::LoginCreatePlayer {
            character_id: self.character_id.0,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::LoginLoginCharacterSet {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::LoginCharacterSet {
            account: self.account.clone(),
            characters: self.characters.list.clone(),
            num_slots: self.num_allowed_characters,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::ItemCreateObject {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::ItemCreateObject {
            object_id: self.object_id.0,
            name: self.weenie_description.name.clone(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CharacterCharacterError {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CharacterError {
            error_code: self.reason.clone() as u32,
            error_message: format!("{}", self.reason),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CommunicationHearSpeech {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::HearSpeech {
            sender_name: self.sender_name.clone(),
            message: self.message.clone(),
            message_type: self.type_.clone() as u32,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CommunicationHearRangedSpeech {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::HearRangedSpeech {
            sender_name: self.sender_name.clone(),
            message: self.message.clone(),
            message_type: self.type_.clone() as u32,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::DDDInterrogationMessage {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::DDDInterrogation {
            language: self.name_rule_language.to_string(),
            region: self.servers_region.to_string(),
            product: self.product_id.to_string(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CharacterCharGenVerificationResponse {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CharGenVerificationResponse
    }
}

// ============================================================================
// Conversion from game event types to GameEventMsg
// ============================================================================
//
// These conversions are used by the game event dispatcher to create protocol events.
// They're implemented as standalone functions to avoid orphan rule issues.

/// Convert acprotocol CommunicationHearDirectSpeech to GameEventMsg
pub fn hear_direct_speech_to_game_event_msg(
    event: acprotocol::gameevents::CommunicationHearDirectSpeech,
) -> GameEventMsg {
    GameEventMsg::HearDirectSpeech {
        message: event.message,
        sender_name: event.sender_name,
        sender_id: event.sender_id.0,
        target_id: event.target_id.0,
        message_type: event.type_ as u32,
    }
}

/// Convert acprotocol CommunicationTransientString to GameEventMsg
pub fn transient_string_to_game_event_msg(
    event: acprotocol::gameevents::CommunicationTransientString,
) -> GameEventMsg {
    GameEventMsg::TransientString {
        message: event.message,
    }
}

#[cfg(test)]
mod tests {
    use gromnie_events::{GameEventMsg, S2CEvent};

    use super::{
        ToProtocolEvent, hear_direct_speech_to_game_event_msg, transient_string_to_game_event_msg,
    };

    /// Test LoginCreatePlayer conversion extracts character_id correctly
    #[test]
    fn test_login_create_player_conversion() {
        let msg = acprotocol::messages::s2c::LoginCreatePlayer {
            character_id: acprotocol::types::ObjectId(0x12345678),
        };

        let result = msg.to_protocol_event();

        match result {
            S2CEvent::LoginCreatePlayer { character_id } => {
                assert_eq!(character_id, 0x12345678);
            }
            _ => panic!("Expected LoginCreatePlayer variant"),
        }
    }

    /// Test CharacterError conversion includes both code and message
    #[test]
    fn test_character_error_conversion() {
        use acprotocol::enums::CharacterErrorType;
        use acprotocol::messages::s2c::CharacterCharacterError;

        let msg = CharacterCharacterError {
            reason: CharacterErrorType::LogonServerFull,
        };

        let result = msg.to_protocol_event();

        match result {
            S2CEvent::CharacterError {
                error_code,
                error_message,
            } => {
                assert_eq!(error_code, CharacterErrorType::LogonServerFull as u32);
                assert!(!error_message.is_empty());
            }
            _ => panic!("Expected CharacterError variant"),
        }
    }

    /// Test HearDirectSpeech game event conversion preserves all fields
    #[test]
    fn test_hear_direct_speech_game_event_conversion() {
        use acprotocol::enums::ChatFragmentType;
        use acprotocol::types::ObjectId;

        let event = acprotocol::gameevents::CommunicationHearDirectSpeech {
            message: "Secret message".to_string(),
            sender_name: "Spy".to_string(),
            sender_id: ObjectId(0x111),
            target_id: ObjectId(0x222),
            type_: ChatFragmentType::Tell,
            secret_flags: 0,
        };

        let result = hear_direct_speech_to_game_event_msg(event);

        match result {
            GameEventMsg::HearDirectSpeech {
                message,
                sender_name,
                sender_id,
                target_id,
                message_type,
            } => {
                assert_eq!(message, "Secret message");
                assert_eq!(sender_name, "Spy");
                assert_eq!(sender_id, 0x111);
                assert_eq!(target_id, 0x222);
                assert_eq!(message_type, ChatFragmentType::Tell as u32);
            }
            _ => panic!("Expected HearDirectSpeech variant"),
        }
    }

    /// Test TransientString game event conversion
    #[test]
    fn test_transient_string_game_event_conversion() {
        let event = acprotocol::gameevents::CommunicationTransientString {
            message: "System notification".to_string(),
        };

        let result = transient_string_to_game_event_msg(event);

        match result {
            GameEventMsg::TransientString { message } => {
                assert_eq!(message, "System notification");
            }
            _ => panic!("Expected TransientString variant"),
        }
    }

    /// Test delete_pending mapping from seconds_greyed_out
    /// This is a critical test that verifies the character deletion status mapping
    #[test]
    fn test_character_delete_pending_mapping() {
        use acprotocol::types::{CharacterIdentity, ObjectId, PackableList};

        let characters = vec![
            CharacterIdentity {
                character_id: ObjectId(0x1),
                name: "ActiveChar".to_string(),
                seconds_greyed_out: 0,
            },
            CharacterIdentity {
                character_id: ObjectId(0x2),
                name: "DeletedChar".to_string(),
                seconds_greyed_out: 86400, // 1 day until deletion
            },
        ];

        let msg = acprotocol::messages::s2c::LoginLoginCharacterSet {
            status: 0,
            characters: PackableList {
                count: characters.len() as u32,
                list: characters,
            },
            deleted_characters: PackableList {
                count: 0,
                list: vec![],
            },
            num_allowed_characters: 5,
            account: "TestAccount".to_string(),
            use_turbine_chat: false,
            has_throneof_destiny: false,
        };

        let result = msg.to_protocol_event();

        match result {
            S2CEvent::LoginCharacterSet {
                account,
                characters,
                num_slots,
            } => {
                assert_eq!(account, "TestAccount");
                assert_eq!(num_slots, 5);
                assert_eq!(characters.len(), 2);

                // Active character should have seconds_greyed_out = 0
                assert_eq!(characters[0].character_id.0, 0x1);
                assert_eq!(characters[0].name, "ActiveChar");
                assert_eq!(characters[0].seconds_greyed_out, 0);

                // Character with seconds_greyed_out > 0 should have the value preserved
                assert_eq!(characters[1].character_id.0, 0x2);
                assert_eq!(characters[1].name, "DeletedChar");
                assert_eq!(characters[1].seconds_greyed_out, 86400);
            }
            _ => panic!("Expected LoginCharacterSet variant"),
        }
    }
}
