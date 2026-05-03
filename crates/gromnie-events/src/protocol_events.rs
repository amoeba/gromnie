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
        characters: Vec<asheron_rs::types::CharacterIdentity>,
        num_slots: u32,
    },
    ItemCreateObject {
        object_id: u32,
        name: String,
        item_type: String,
        container_id: Option<u32>,
        burden: u32,
        value: u32,
        items_capacity: Option<u32>,
        container_capacity: Option<u32>,
    },
    ItemDeleteObject {
        object_id: u32,
    },
    ItemOnViewContents {
        container_id: u32,
        items: Vec<u32>,
    },
    PlayerContainersReceived {
        player_id: u32,
        containers: Vec<u32>,
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
    ItemSetState {
        object_id: u32,
        state: String,
    },
    QualitiesPrivateUpdateInt {
        property: String,
        value: i32,
    },
    /// Movement_PositionEvent (0xF748) - position/motion update for an object
    MovementPositionEvent {
        object_id: u32,
        landcell: u32,
        x: f32,
        y: f32,
        z: f32,
        quat_w: Option<f32>,
        quat_x: Option<f32>,
        quat_y: Option<f32>,
        quat_z: Option<f32>,
    },
    /// Movement_PositionAndMovementEvent (0xF619) - position + movement (e.g. lifestone recall)
    MovementPositionAndMovementEvent {
        object_id: u32,
        landcell: u32,
        x: f32,
        y: f32,
        z: f32,
        quat_w: Option<f32>,
        quat_x: Option<f32>,
        quat_y: Option<f32>,
        quat_z: Option<f32>,
    },
    /// Movement_SetObjectMovement (0xF74C) - animation/movement state for an object
    MovementSetObjectMovement {
        object_id: u32,
        object_instance_sequence: u16,
    },
    /// Effects_PlayerTeleport (0xF751) - server signals a teleport occurred
    EffectsPlayerTeleport {
        object_teleport_sequence: u16,
    },
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

    // ===== Trading Events =====
    /// Server confirmed trade registration (both players notified)
    TradeRegistered {
        initiator_id: u32,
        partner_id: u32,
        stamp: i64,
    },
    /// Trade window was opened on this character's side
    TradeOpened {
        object_id: u32,
    },
    /// Trade was closed (window closed)
    TradeClosed,
    /// An item was added to the trade window (by either side)
    TradeItemAdded {
        item_id: u32,
    },
    /// An item was removed from the trade window
    TradeItemRemoved {
        item_id: u32,
    },
    /// A trade participant accepted the trade
    TradeAccepted,
    /// A trade participant declined the trade
    TradeDeclined,
    /// Trade was reset (items removed, acceptance cleared)
    TradeReset,
    /// Trade failed for some reason
    TradeFailure {
        reason: u32,
    },

    // ===== Spell / Enchantment Events =====
    /// An enchantment (buff/debuff) was applied or refreshed on this character
    EnchantmentUpdated {
        spell_id: u32,
        duration: f64,
        caster_id: u32,
        power_level: u32,
    },
    /// An enchantment was removed from this character
    EnchantmentRemoved {
        spell_id: u32,
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
