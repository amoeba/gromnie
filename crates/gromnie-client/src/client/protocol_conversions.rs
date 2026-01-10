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
            item_type: format!("{:?}", self.weenie_description.type_),
            container_id: self.weenie_description.container_id.map(|id| id.0),
            burden: self.weenie_description.burden.unwrap_or(0) as u32,
            value: self.weenie_description.value.unwrap_or(0),
            items_capacity: self.weenie_description.items_capacity.map(|c| c as u32),
            container_capacity: self.weenie_description.container_capacity.map(|c| c as u32),
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

impl ToProtocolEvent for acprotocol::messages::s2c::ItemSetState {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::ItemSetState {
            object_id: self.object_id.0,
            state: format!("{:?}", self.new_state),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateInt {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateInt {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::ItemDeleteObject {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::ItemDeleteObject {
            object_id: self.object_id.0,
        }
    }
}

// ============================================================================
// Quality/Property Update Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateInt {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateInt {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateInt64 {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateInt64 {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateInt64 {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateInt64 {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateBool {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateBool {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateBool {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateBool {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateFloat {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateFloat {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value as f64,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateFloat {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateFloat {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value as f64,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateString {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateString {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value.clone(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateString {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateString {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value.clone(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateDataId {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateDataId {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateDataId {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateDataId {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesUpdateInstanceId {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesUpdateInstanceId {
            sequence: self.sequence,
            object_id: self.object_id.0,
            property: format!("{:?}", self.key),
            value: self.value.0,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::QualitiesPrivateUpdateInstanceId {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::QualitiesPrivateUpdateInstanceId {
            sequence: self.sequence,
            property: format!("{:?}", self.key),
            value: self.value.0,
        }
    }
}

// ============================================================================
// Communication Message Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::CommunicationHearEmote {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CommunicationHearEmote {
            sender_name: self.sender_name.clone(),
            message: self.text.clone(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CommunicationHearSoulEmote {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CommunicationHearSoulEmote {
            sender_name: self.sender_name.clone(),
            message: self.text.clone(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::CommunicationTextboxString {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CommunicationTextboxString {
            message: self.text.clone(),
            message_type: self.type_.clone() as u32,
        }
    }
}

// ============================================================================
// Item/Inventory Message Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::ItemUpdateStackSize {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::ItemUpdateStackSize {
            object_id: self.object_id.0,
            stack_size: self.new_value as i32,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::ItemServerSaysRemove {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::ItemServerSaysRemove {
            object_id: self.object_id.0,
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::InventoryPickupEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::InventoryPickup {
            object_id: self.object_id.0,
        }
    }
}

// ============================================================================
// Effects Message Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::EffectsSoundEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::EffectsSoundEvent {
            object_id: self.object_id.0,
            sound_id: self.sound_type.clone() as u32,
        }
    }
}

// ============================================================================
// Movement Message Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::MovementPositionAndMovementEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::MovementPositionAndMovement {
            object_id: self.object_id.0,
            position: serde_json::to_string(&self.position).unwrap_or_default(),
            movement_data: serde_json::to_string(&self.movement_data).unwrap_or_default(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::MovementPositionEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::MovementPosition {
            object_id: self.object_id.0,
            position: serde_json::to_string(&self.position).unwrap_or_default(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::MovementSetObjectMovement {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::MovementSetObjectMovement {
            object_id: self.object_id.0,
            instance_sequence: self.object_instance_sequence,
            movement_data: serde_json::to_string(&self.movement_data).unwrap_or_default(),
        }
    }
}

impl ToProtocolEvent for acprotocol::messages::s2c::MovementVectorUpdate {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::MovementVectorUpdate {
            object_id: self.object_id.0,
            velocity_x: self.velocity.x,
            velocity_y: self.velocity.y,
            velocity_z: self.velocity.z,
            omega_x: self.omega.x,
            omega_y: self.omega.y,
            omega_z: self.omega.z,
            instance_sequence: self.object_instance_sequence,
            vector_sequence: self.object_vector_sequence,
        }
    }
}

// ============================================================================
// Combat Message Conversions
// ============================================================================

impl ToProtocolEvent for acprotocol::messages::s2c::CombatHandlePlayerDeathEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CombatHandlePlayerDeath {
            victim_id: self.killed_id.0,
            killer_id: self.killer_id.0,
        }
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

// ============================================================================
// Additional Game Event Conversions
// ============================================================================

/// Convert acprotocol ItemOnViewContents to GameEventMsg
pub fn item_on_view_contents_to_game_event_msg(
    event: acprotocol::gameevents::ItemOnViewContents,
) -> GameEventMsg {
    GameEventMsg::ItemOnViewContents {
        container_id: event.container_id.0,
        items: event.items.list.iter().map(|cp| cp.object_id.0).collect(),
    }
}

/// Convert acprotocol MagicUpdateSpell to GameEventMsg
pub fn magic_update_spell_to_game_event_msg(
    event: acprotocol::gameevents::MagicUpdateSpell,
) -> GameEventMsg {
    GameEventMsg::MagicUpdateSpell {
        spell_id: event.spell_id.id.0 as u32,
    }
}

/// Convert acprotocol FellowshipFullUpdate to GameEventMsg
pub fn fellowship_full_update_to_game_event_msg(
    event: acprotocol::gameevents::FellowshipFullUpdate,
) -> GameEventMsg {
    let fellowship = &event.fellowship;
    let members: Vec<u32> = fellowship.members.table.keys().map(|id| id.0).collect();
    GameEventMsg::FellowshipFullUpdate {
        fellowship_id: fellowship.leader_id.0, // Use leader_id as fellowship_id
        leader_name: fellowship.name.clone(),
        leader_id: fellowship.leader_id.0,
        members,
        locked: fellowship.locked,
    }
}

/// Convert acprotocol TradeRegisterTrade to GameEventMsg
pub fn trade_register_trade_to_game_event_msg(
    event: acprotocol::gameevents::TradeRegisterTrade,
) -> GameEventMsg {
    GameEventMsg::TradeRegisterTrade {
        initiator_id: event.initiator_id.0,
        partner_id: event.partner_id.0,
    }
}

// ============================================================================
// Combat Game Event Conversions
// ============================================================================

/// Convert acprotocol CombatHandleAttackDoneEvent to GameEventMsg
pub fn combat_handle_attack_done_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleAttackDoneEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleAttackDone {
        target_id: event.number,
    }
}

/// Convert acprotocol CombatHandleCommenceAttackEvent to GameEventMsg
pub fn combat_handle_commence_attack_to_game_event_msg(
    _event: acprotocol::gameevents::CombatHandleCommenceAttackEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleCommenceAttack {
        target_id: 0, // This event has no fields
    }
}

/// Convert acprotocol CombatHandleVictimNotificationEventSelf to GameEventMsg
pub fn combat_handle_victim_notification_self_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleVictimNotificationEventSelf,
) -> GameEventMsg {
    GameEventMsg::CombatHandleVictimNotificationSelf {
        message: event.message,
        damage_type: 0, // Not available in event
        damage_amount: 0, // Not available in event
        critical: false, // Not available in event
    }
}

/// Convert acprotocol CombatHandleVictimNotificationEventOther to GameEventMsg
pub fn combat_handle_victim_notification_other_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleVictimNotificationEventOther,
) -> GameEventMsg {
    GameEventMsg::CombatHandleVictimNotificationOther {
        attacker_name: event.message,
        part_index: 0, // Not available in event
    }
}

/// Convert acprotocol CombatHandleAttackerNotificationEvent to GameEventMsg
pub fn combat_handle_attacker_notification_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleAttackerNotificationEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleAttackerNotification {
        victim_name: event.defender_name,
        damage_type: event.type_.bits(),
        damage_amount: event.damage as f32,
        critical: event.critical,
        hit_location: event.attack_conditions.bits(),
    }
}

/// Convert acprotocol CombatHandleDefenderNotificationEvent to GameEventMsg
pub fn combat_handle_defender_notification_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleDefenderNotificationEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleDefenderNotification {
        attacker_name: event.attacker_name,
        damage_type: event.type_.bits(),
        evasion_result: event.attack_conditions.bits(),
    }
}

/// Convert acprotocol CombatHandleEvasionAttackerNotificationEvent to GameEventMsg
pub fn combat_handle_evasion_attacker_notification_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleEvasionAttackerNotificationEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleEvasionAttackerNotification {
        defender_name: event.defender_name,
        evasion_result: 0, // Not available in event
    }
}

/// Convert acprotocol CombatHandleEvasionDefenderNotificationEvent to GameEventMsg
pub fn combat_handle_evasion_defender_notification_to_game_event_msg(
    event: acprotocol::gameevents::CombatHandleEvasionDefenderNotificationEvent,
) -> GameEventMsg {
    GameEventMsg::CombatHandleEvasionDefenderNotification {
        attacker_name: event.attacker_name,
        evasion_result: 0, // Not available in event
    }
}

/// Convert acprotocol CombatQueryHealthResponse to GameEventMsg
pub fn combat_query_health_response_to_game_event_msg(
    event: acprotocol::gameevents::CombatQueryHealthResponse,
) -> GameEventMsg {
    GameEventMsg::CombatQueryHealthResponse {
        target_id: event.object_id.0,
        health_percent: event.health,
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
