//! Protocol event types - strongly-typed wrappers for acprotocol events
//!
//! This module defines Rust types that mirror the WIT protocol event types,
//! providing full access to the acprotocol event stream with type safety.

use serde::{Deserialize, Serialize};

/// Protocol event - mirrors WIT structure
///
/// This is the unified wrapper for all protocol events from the server,
/// both top-level S2C messages and nested game events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProtocolEvent {
    S2C(S2CEvent),
    GameEvent(OrderedGameEvent),
}

/// Top-level S2C message events
///
/// These correspond directly to acprotocol server-to-client message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum S2CEvent {
    LoginCreatePlayer {
        character_id: u32,
    },
    LoginCharacterSet {
        account: String,
        characters: Vec<acprotocol::types::CharacterIdentity>,
        num_slots: u32,
    },
    ItemCreateObject {
        object_id: u32,
        name: String,
    },
    CharacterError {
        error_code: u32,
        error_message: String,
    },
    HearSpeech {
        sender_name: String,
        message: String,
        message_type: u32,
    },
    HearRangedSpeech {
        sender_name: String,
        message: String,
        message_type: u32,
    },
    DDDInterrogation {
        language: String,
        region: String,
        product: String,
    },
    CharGenVerificationResponse,
}

/// Nested game events with OrderedGameEvent metadata
///
/// These are events contained within the OrderedGameEvent wrapper (0xF7B0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedGameEvent {
    pub object_id: u32,
    pub sequence: u32,
    pub event: GameEventMsg,
}

/// Game event messages (nested within OrderedGameEvent)
///
/// These correspond to the individual game event types from acprotocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GameEventMsg {
    HearDirectSpeech {
        message: String,
        sender_name: String,
        sender_id: u32,
        target_id: u32,
        message_type: u32,
    },
    TransientString {
        message: String,
    },
}

// ============================================================================
// Conversion from game event handler types
// ============================================================================

/// Convert from CommunicationHearDirectSpeech game event handler type
///
/// This type is defined in gromnie-client's game_event_handlers module.
pub trait IntoGameEventMsg {
    fn into_game_event_msg(self) -> GameEventMsg;
}

// Note: The actual From implementations for acprotocol types are in
// gromnie-client/src/client/protocol_conversions.rs to avoid circular
// dependencies, since gromnie-events doesn't depend on acprotocol.
