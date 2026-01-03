//! Conversion implementations from acprotocol types to ProtocolEvent types
//!
//! This module contains helper functions that convert acprotocol message
//! types into our strongly-typed ProtocolEvent wrapper types. These are here
//! (instead of gromnie-events) to avoid circular dependencies, since only
//! gromnie-client depends on acprotocol.

use gromnie_events::{CharacterData, GameEventMsg, S2CEvent};

/// Helper trait for converting acprotocol types to S2CEvent
pub trait ToS2CEvent {
    fn to_s2c_event(&self) -> S2CEvent;
}

impl ToS2CEvent for acprotocol::messages::s2c::LoginCreatePlayer {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::LoginCreatePlayer {
            character_id: self.character_id.0,
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::LoginLoginCharacterSet {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::LoginCharacterSet {
            account: self.account.clone(),
            characters: self.characters.list.iter().map(|c| CharacterData {
                id: c.character_id.0,
                name: c.name.clone(),
                delete_pending: c.seconds_greyed_out > 0,
            }).collect(),
            num_slots: self.num_allowed_characters,
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::ItemCreateObject {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::ItemCreateObject {
            object_id: self.object_id.0,
            name: self.weenie_description.name.clone(),
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::CharacterCharacterError {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::CharacterError {
            error_code: self.reason.clone() as u32,
            error_message: format!("{}", self.reason),
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::CommunicationHearSpeech {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::HearSpeech {
            sender_name: self.sender_name.clone(),
            message: self.message.clone(),
            message_type: self.type_.clone() as u32,
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::CommunicationHearRangedSpeech {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::HearRangedSpeech {
            sender_name: self.sender_name.clone(),
            message: self.message.clone(),
            message_type: self.type_.clone() as u32,
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::DDDInterrogationMessage {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::DDDInterrogation {
            language: self.name_rule_language.to_string(),
            region: self.servers_region.to_string(),
            product: self.product_id.to_string(),
        }
    }
}

impl ToS2CEvent for acprotocol::messages::s2c::CharacterCharGenVerificationResponse {
    fn to_s2c_event(&self) -> S2CEvent {
        S2CEvent::CharGenVerificationResponse
    }
}

// ============================================================================
// Conversion from game event handler types
// ============================================================================

impl gromnie_events::IntoGameEventMsg for crate::client::game_event_handlers::CommunicationHearDirectSpeech {
    fn into_game_event_msg(self) -> GameEventMsg {
        GameEventMsg::HearDirectSpeech {
            message: self.message,
            sender_name: self.sender_name,
            sender_id: self.sender_id,
            target_id: self.target_id,
            message_type: self.message_type,
        }
    }
}

impl gromnie_events::IntoGameEventMsg
    for crate::client::game_event_handlers::CommunicationTransientString
{
    fn into_game_event_msg(self) -> GameEventMsg {
        GameEventMsg::TransientString {
            message: self.message,
        }
    }
}
