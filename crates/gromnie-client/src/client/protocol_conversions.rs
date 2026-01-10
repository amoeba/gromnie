//! Conversion implementations from acprotocol types to ProtocolEvent types
//!
//! This module contains helper functions that convert acprotocol message
//! types into our strongly-typed ProtocolEvent wrapper types. These are here
//! (instead of gromnie-events) to avoid circular dependencies, since only
//! gromnie-client depends on acprotocol.

use gromnie_events::{GameEventMsg, S2CEvent};

pub mod prelude {
    //! Re-export all game event conversion functions for convenient importing
    pub use super::{
        admin_query_plugin_list_to_game_event_msg, admin_query_plugin_response_to_game_event_msg,
        admin_query_plugin_to_game_event_msg, allegiance_info_response_to_game_event_msg,
        allegiance_login_notification_to_game_event_msg,
        allegiance_update_aborted_to_game_event_msg, allegiance_update_done_to_game_event_msg,
        allegiance_update_to_game_event_msg, channel_broadcast_to_game_event_msg,
        channel_index_to_game_event_msg, channel_list_to_game_event_msg,
        character_confirmation_request_to_game_event_msg,
        character_query_age_response_to_game_event_msg, character_return_ping_to_game_event_msg,
        character_start_barber_to_game_event_msg, combat_handle_attack_done_to_game_event_msg,
        combat_handle_attacker_notification_to_game_event_msg,
        combat_handle_commence_attack_to_game_event_msg,
        combat_handle_defender_notification_to_game_event_msg,
        combat_handle_evasion_attacker_notification_to_game_event_msg,
        combat_handle_evasion_defender_notification_to_game_event_msg,
        combat_handle_victim_notification_other_to_game_event_msg,
        combat_handle_victim_notification_self_to_game_event_msg,
        combat_query_health_response_to_game_event_msg,
        communication_chat_room_tracker_to_game_event_msg,
        communication_popup_string_to_game_event_msg,
        communication_set_squelch_db_to_game_event_msg,
        communication_weenie_error_to_game_event_msg,
        communication_weenie_error_with_string_to_game_event_msg,
        fellowship_disband_to_game_event_msg, fellowship_dismiss_to_game_event_msg,
        fellowship_full_update_to_game_event_msg, fellowship_quit_to_game_event_msg,
        fellowship_stats_done_to_game_event_msg, fellowship_update_done_to_game_event_msg,
        fellowship_update_fellow_to_game_event_msg, game_game_over_to_game_event_msg,
        game_join_game_response_to_game_event_msg, game_move_response_to_game_event_msg,
        game_opponent_stalemate_state_to_game_event_msg, game_opponent_turn_to_game_event_msg,
        game_start_game_to_game_event_msg, hear_direct_speech_to_game_event_msg,
        house_available_houses_to_game_event_msg, house_data_to_game_event_msg,
        house_profile_to_game_event_msg, house_status_to_game_event_msg,
        house_transaction_to_game_event_msg, house_update_har_to_game_event_msg,
        house_update_rent_payment_to_game_event_msg, house_update_rent_time_to_game_event_msg,
        house_update_restrictions_to_game_event_msg,
        inventory_salvage_operations_result_to_game_event_msg,
        item_appraise_done_to_game_event_msg, item_on_view_contents_to_game_event_msg,
        item_query_item_mana_response_to_game_event_msg, item_set_appraise_info_to_game_event_msg,
        item_wear_item_to_game_event_msg, login_player_description_to_game_event_msg,
        magic_remove_enchantment_to_game_event_msg, magic_update_enchantment_to_game_event_msg,
        magic_update_spell_to_game_event_msg, misc_portal_storm_brewing_to_game_event_msg,
        misc_portal_storm_imminent_to_game_event_msg, misc_portal_storm_subsided_to_game_event_msg,
        misc_portal_storm_to_game_event_msg, social_add_or_set_character_title_to_game_event_msg,
        social_character_title_table_to_game_event_msg, social_friends_update_to_game_event_msg,
        social_send_client_contract_tracker_table_to_game_event_msg,
        social_send_client_contract_tracker_to_game_event_msg,
        trade_accept_trade_to_game_event_msg, trade_add_to_trade_to_game_event_msg,
        trade_clear_trade_acceptance_to_game_event_msg, trade_close_trade_to_game_event_msg,
        trade_decline_trade_to_game_event_msg, trade_open_trade_to_game_event_msg,
        trade_register_trade_to_game_event_msg, trade_remove_from_trade_to_game_event_msg,
        trade_reset_trade_to_game_event_msg, trade_trade_failure_to_game_event_msg,
        transient_string_to_game_event_msg, vendor_info_to_game_event_msg,
        writing_book_add_page_response_to_game_event_msg,
        writing_book_delete_page_response_to_game_event_msg, writing_book_open_to_game_event_msg,
        writing_book_page_data_response_to_game_event_msg,
    };
}

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

// Quality/Property Update Conversions

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

// Communication Message Conversions

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

// Item/Inventory Message Conversions

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

// Effects Message Conversions

impl ToProtocolEvent for acprotocol::messages::s2c::EffectsSoundEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::EffectsSoundEvent {
            object_id: self.object_id.0,
            sound_id: self.sound_type.clone() as u32,
        }
    }
}

// Movement Message Conversions

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

// Combat Message Conversions

impl ToProtocolEvent for acprotocol::messages::s2c::CombatHandlePlayerDeathEvent {
    fn to_protocol_event(&self) -> S2CEvent {
        S2CEvent::CombatHandlePlayerDeath {
            victim_id: self.killed_id.0,
            killer_id: self.killer_id.0,
        }
    }
}

// Conversion from game event types to GameEventMsg
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

// Additional Game Event Conversions

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

// Combat Game Event Conversions

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
        damage_type: 0,   // Not available in event
        damage_amount: 0, // Not available in event
        critical: false,  // Not available in event
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

// Magic & Items Conversions

/// Convert acprotocol MagicUpdateEnchantment game event to GameEventMsg
pub fn magic_update_enchantment_to_game_event_msg(
    event: acprotocol::gameevents::MagicUpdateEnchantment,
) -> GameEventMsg {
    GameEventMsg::MagicUpdateEnchantment {
        enchantment_id: event.enchantment.id.id.0,
        spell_id: event.enchantment.id.id.0 as u32,
        layer: event.enchantment.id.layer,
        caster_id: event.enchantment.caster_id.0,
        power_level: event.enchantment.power_level as f32,
        start_time: event.enchantment.start_time,
        duration: event.enchantment.duration,
    }
}

/// Convert acprotocol MagicRemoveEnchantment game event to GameEventMsg
pub fn magic_remove_enchantment_to_game_event_msg(
    event: acprotocol::gameevents::MagicRemoveEnchantment,
) -> GameEventMsg {
    GameEventMsg::MagicRemoveEnchantment {
        enchantment_id: event.spell_id.id.0,
    }
}

/// Convert acprotocol ItemSetAppraiseInfo game event to GameEventMsg
pub fn item_set_appraise_info_to_game_event_msg(
    event: acprotocol::gameevents::ItemSetAppraiseInfo,
) -> GameEventMsg {
    GameEventMsg::ItemSetAppraiseInfo {
        object_id: event.object_id.0,
        success: event.success,
        appraisal_data: vec![], // Complex nested structure, storing as empty for now
    }
}

/// Convert acprotocol ItemAppraiseDone game event to GameEventMsg
pub fn item_appraise_done_to_game_event_msg(
    _event: acprotocol::gameevents::ItemAppraiseDone,
) -> GameEventMsg {
    GameEventMsg::ItemAppraiseDone {
        object_id: 0, // Unknown field in acprotocol event
    }
}

/// Convert acprotocol ItemWearItem game event to GameEventMsg
pub fn item_wear_item_to_game_event_msg(
    event: acprotocol::gameevents::ItemWearItem,
) -> GameEventMsg {
    GameEventMsg::ItemWearItem {
        object_id: event.object_id.0,
        equipped_slot: event.slot.bits(),
    }
}

/// Convert acprotocol ItemQueryItemManaResponse game event to GameEventMsg
pub fn item_query_item_mana_response_to_game_event_msg(
    event: acprotocol::gameevents::ItemQueryItemManaResponse,
) -> GameEventMsg {
    GameEventMsg::ItemQueryItemManaResponse {
        object_id: event.object_id.0,
        mana: event.mana,
        max_mana: 100, // Not available in event, using placeholder
    }
}

// Trade Game Event Conversions

/// Convert acprotocol TradeOpenTrade to GameEventMsg
pub fn trade_open_trade_to_game_event_msg(
    _event: acprotocol::gameevents::TradeOpenTrade,
) -> GameEventMsg {
    // TradeOpenTrade only contains object_id, partner name/id not available
    GameEventMsg::TradeOpenTrade {
        partner_id: 0,
        partner_name: "Unknown".to_string(),
    }
}

/// Convert acprotocol TradeCloseTrade to GameEventMsg
pub fn trade_close_trade_to_game_event_msg(
    event: acprotocol::gameevents::TradeCloseTrade,
) -> GameEventMsg {
    GameEventMsg::TradeCloseTrade {
        reason: event.reason.clone() as u32,
    }
}

/// Convert acprotocol TradeAddToTrade to GameEventMsg
pub fn trade_add_to_trade_to_game_event_msg(
    event: acprotocol::gameevents::TradeAddToTrade,
) -> GameEventMsg {
    GameEventMsg::TradeAddToTrade {
        item_id: event.object_id.0,
        trader_id: 0, // Not available in event
    }
}

/// Convert acprotocol TradeRemoveFromTrade to GameEventMsg
pub fn trade_remove_from_trade_to_game_event_msg(
    event: acprotocol::gameevents::TradeRemoveFromTrade,
) -> GameEventMsg {
    GameEventMsg::TradeRemoveFromTrade {
        item_id: event.object_id.0,
        trader_id: 0, // Not available in event
    }
}

/// Convert acprotocol TradeAcceptTrade to GameEventMsg
pub fn trade_accept_trade_to_game_event_msg(
    event: acprotocol::gameevents::TradeAcceptTrade,
) -> GameEventMsg {
    GameEventMsg::TradeAcceptTrade {
        trader_id: event.object_id.0,
    }
}

/// Convert acprotocol TradeDeclineTrade to GameEventMsg
pub fn trade_decline_trade_to_game_event_msg(
    _event: acprotocol::gameevents::TradeDeclineTrade,
) -> GameEventMsg {
    GameEventMsg::TradeDeclineTrade {}
}

/// Convert acprotocol TradeResetTrade to GameEventMsg
pub fn trade_reset_trade_to_game_event_msg(
    _event: acprotocol::gameevents::TradeResetTrade,
) -> GameEventMsg {
    GameEventMsg::TradeResetTrade {}
}

/// Convert acprotocol TradeTradeFailure to GameEventMsg
pub fn trade_trade_failure_to_game_event_msg(
    event: acprotocol::gameevents::TradeTradeFailure,
) -> GameEventMsg {
    GameEventMsg::TradeTradeFailure {
        reason: event.reason,
    }
}

/// Convert acprotocol TradeClearTradeAcceptance to GameEventMsg
pub fn trade_clear_trade_acceptance_to_game_event_msg(
    _event: acprotocol::gameevents::TradeClearTradeAcceptance,
) -> GameEventMsg {
    GameEventMsg::TradeClearTradeAcceptance {}
}

// Fellowship Game Event Conversions

/// Convert acprotocol FellowshipUpdateFellow to GameEventMsg
pub fn fellowship_update_fellow_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipUpdateFellow,
) -> GameEventMsg {
    // FellowshipUpdateFellow contains Fellow struct - simplified stub
    GameEventMsg::FellowshipUpdateFellow {
        fellow_id: 0,
        fellow_name: String::new(),
        health_percent: 0.0,
        stamina_percent: 0.0,
        mana_percent: 0.0,
        level: 0,
    }
}

/// Convert acprotocol FellowshipDisband to GameEventMsg
pub fn fellowship_disband_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipDisband,
) -> GameEventMsg {
    GameEventMsg::FellowshipDisband {}
}

/// Convert acprotocol FellowshipQuit to GameEventMsg
pub fn fellowship_quit_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipQuit,
) -> GameEventMsg {
    GameEventMsg::FellowshipQuit { fellow_id: 0 }
}

/// Convert acprotocol FellowshipDismiss to GameEventMsg
pub fn fellowship_dismiss_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipDismiss,
) -> GameEventMsg {
    GameEventMsg::FellowshipDismiss { fellow_id: 0 }
}

/// Convert acprotocol FellowshipFellowUpdateDone to GameEventMsg
pub fn fellowship_update_done_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipFellowUpdateDone,
) -> GameEventMsg {
    GameEventMsg::FellowshipUpdateDone {}
}

/// Convert acprotocol FellowshipFellowStatsDone to GameEventMsg
pub fn fellowship_stats_done_to_game_event_msg(
    _event: acprotocol::gameevents::FellowshipFellowStatsDone,
) -> GameEventMsg {
    GameEventMsg::FellowshipStatsDone {}
}

// Social Game Event Conversions

/// Convert acprotocol SocialFriendsUpdate to GameEventMsg
pub fn social_friends_update_to_game_event_msg(
    _event: acprotocol::gameevents::SocialFriendsUpdate,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::SocialFriendsUpdate { friends: vec![] }
}

/// Convert acprotocol SocialCharacterTitleTable to GameEventMsg
pub fn social_character_title_table_to_game_event_msg(
    _event: acprotocol::gameevents::SocialCharacterTitleTable,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::SocialCharacterTitleTable { titles: vec![] }
}

/// Convert acprotocol SocialAddOrSetCharacterTitle to GameEventMsg
pub fn social_add_or_set_character_title_to_game_event_msg(
    _event: acprotocol::gameevents::SocialAddOrSetCharacterTitle,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::SocialAddOrSetCharacterTitle {
        title_id: 0,
        title: String::new(),
        active: false,
    }
}

/// Convert acprotocol SocialSendClientContractTrackerTable to GameEventMsg
pub fn social_send_client_contract_tracker_table_to_game_event_msg(
    _event: acprotocol::gameevents::SocialSendClientContractTrackerTable,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::SocialSendClientContractTrackerTable { contracts: vec![] }
}

/// Convert acprotocol SocialSendClientContractTracker to GameEventMsg
pub fn social_send_client_contract_tracker_to_game_event_msg(
    _event: acprotocol::gameevents::SocialSendClientContractTracker,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::SocialSendClientContractTracker {
        contract_id: 0,
        stage: 0,
        timestamp: 0,
    }
}

// Allegiance Game Event Conversions

/// Convert acprotocol AllegianceAllegianceUpdate to GameEventMsg
pub fn allegiance_update_to_game_event_msg(
    _event: acprotocol::gameevents::AllegianceAllegianceUpdate,
) -> GameEventMsg {
    // Serialize allegiance data - simplified for now
    GameEventMsg::AllegianceUpdate {
        allegiance_data: vec![], // Complex structure, empty for now
    }
}

/// Convert acprotocol AllegianceAllegianceUpdateDone to GameEventMsg
pub fn allegiance_update_done_to_game_event_msg(
    _event: acprotocol::gameevents::AllegianceAllegianceUpdateDone,
) -> GameEventMsg {
    GameEventMsg::AllegianceUpdateDone {}
}

/// Convert acprotocol AllegianceAllegianceUpdateAborted to GameEventMsg
pub fn allegiance_update_aborted_to_game_event_msg(
    _event: acprotocol::gameevents::AllegianceAllegianceUpdateAborted,
) -> GameEventMsg {
    GameEventMsg::AllegianceUpdateAborted {}
}

/// Convert acprotocol AllegianceAllegianceLoginNotificationEvent to GameEventMsg
pub fn allegiance_login_notification_to_game_event_msg(
    _event: acprotocol::gameevents::AllegianceAllegianceLoginNotificationEvent,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::AllegianceLoginNotification {
        member_name: String::new(),
        member_id: 0,
    }
}

/// Convert acprotocol AllegianceInfoResponse to GameEventMsg
pub fn allegiance_info_response_to_game_event_msg(
    _event: acprotocol::gameevents::AllegianceAllegianceInfoResponseEvent,
) -> GameEventMsg {
    // Simplified stub - complex structure
    GameEventMsg::AllegianceInfoResponse {
        allegiance_name: String::new(),
        allegiance_data: vec![],
    }
}

// Vendor Game Event Conversions

/// Convert acprotocol VendorVendorInfo to GameEventMsg
pub fn vendor_info_to_game_event_msg(
    _event: acprotocol::gameevents::VendorVendorInfo,
) -> GameEventMsg {
    // Simplified stub - complex ItemProfile structure
    GameEventMsg::VendorInfo {
        vendor_id: 0,
        vendor_type: 0,
        items: vec![],
    }
}

// Housing Game Event Conversions

/// Convert acprotocol HouseHouseProfile to GameEventMsg
pub fn house_profile_to_game_event_msg(
    _event: acprotocol::gameevents::HouseHouseProfile,
) -> GameEventMsg {
    GameEventMsg::HouseProfile {
        house_id: 0,
        owner_id: 0,
        house_type: 0,
    }
}

/// Convert acprotocol HouseHouseData to GameEventMsg
pub fn house_data_to_game_event_msg(
    _event: acprotocol::gameevents::HouseHouseData,
) -> GameEventMsg {
    GameEventMsg::HouseData {
        house_id: 0,
        position: vec![],
    }
}

/// Convert acprotocol HouseHouseStatus to GameEventMsg
pub fn house_status_to_game_event_msg(
    _event: acprotocol::gameevents::HouseHouseStatus,
) -> GameEventMsg {
    GameEventMsg::HouseStatus {
        house_id: 0,
        status: 0,
    }
}

/// Convert acprotocol HouseHouseUpdateRentTime to GameEventMsg
pub fn house_update_rent_time_to_game_event_msg(
    _event: acprotocol::gameevents::HouseUpdateRentTime,
) -> GameEventMsg {
    GameEventMsg::HouseUpdateRentTime {
        house_id: 0,
        rent_time: 0,
    }
}

/// Convert acprotocol HouseHouseUpdateRentPayment to GameEventMsg
pub fn house_update_rent_payment_to_game_event_msg(
    _event: acprotocol::gameevents::HouseUpdateRentPayment,
) -> GameEventMsg {
    GameEventMsg::HouseUpdateRentPayment {
        house_id: 0,
        payment: 0,
    }
}

/// Convert acprotocol HouseHouseUpdateRestrictions to GameEventMsg
pub fn house_update_restrictions_to_game_event_msg(
    _event: acprotocol::gameevents::HouseUpdateRestrictions,
) -> GameEventMsg {
    GameEventMsg::HouseUpdateRestrictions {
        house_id: 0,
        restrictions: 0,
    }
}

/// Convert acprotocol HouseHouseUpdateHAR to GameEventMsg
pub fn house_update_har_to_game_event_msg(
    _event: acprotocol::gameevents::HouseUpdateHAR,
) -> GameEventMsg {
    GameEventMsg::HouseUpdateHAR {
        house_id: 0,
        har: vec![],
    }
}

/// Convert acprotocol HouseHouseTransaction to GameEventMsg
pub fn house_transaction_to_game_event_msg(
    _event: acprotocol::gameevents::HouseHouseTransaction,
) -> GameEventMsg {
    GameEventMsg::HouseTransaction {
        house_id: 0,
        transaction_type: 0,
    }
}

/// Convert acprotocol HouseHouseAvailableHouses to GameEventMsg
pub fn house_available_houses_to_game_event_msg(
    _event: acprotocol::gameevents::HouseAvailableHouses,
) -> GameEventMsg {
    GameEventMsg::HouseAvailableHouses { houses: vec![] }
}

// Writing Game Event Conversions

/// Convert acprotocol WritingWritingBookOpen to GameEventMsg
pub fn writing_book_open_to_game_event_msg(
    _event: acprotocol::gameevents::WritingBookOpen,
) -> GameEventMsg {
    GameEventMsg::WritingBookOpen {
        book_id: 0,
        pages: 0,
    }
}

/// Convert acprotocol WritingWritingBookAddPageResponse to GameEventMsg
pub fn writing_book_add_page_response_to_game_event_msg(
    _event: acprotocol::gameevents::WritingBookAddPageResponse,
) -> GameEventMsg {
    GameEventMsg::WritingBookAddPageResponse {
        book_id: 0,
        success: false,
    }
}

/// Convert acprotocol WritingWritingBookDeletePageResponse to GameEventMsg
pub fn writing_book_delete_page_response_to_game_event_msg(
    _event: acprotocol::gameevents::WritingBookDeletePageResponse,
) -> GameEventMsg {
    GameEventMsg::WritingBookDeletePageResponse {
        book_id: 0,
        success: false,
    }
}

/// Convert acprotocol WritingWritingBookPageDataResponse to GameEventMsg
pub fn writing_book_page_data_response_to_game_event_msg(
    _event: acprotocol::gameevents::WritingBookPageDataResponse,
) -> GameEventMsg {
    GameEventMsg::WritingBookPageDataResponse {
        book_id: 0,
        page: 0,
        content: String::new(),
    }
}

// Character Game Event Conversions

/// Convert acprotocol CharacterCharacterStartBarber to GameEventMsg
pub fn character_start_barber_to_game_event_msg(
    _event: acprotocol::gameevents::CharacterStartBarber,
) -> GameEventMsg {
    GameEventMsg::CharacterStartBarber { barber_id: 0 }
}

/// Convert acprotocol CharacterCharacterQueryAgeResponse to GameEventMsg
pub fn character_query_age_response_to_game_event_msg(
    _event: acprotocol::gameevents::CharacterQueryAgeResponse,
) -> GameEventMsg {
    GameEventMsg::CharacterQueryAgeResponse { age: 0 }
}

/// Convert acprotocol CharacterCharacterConfirmationRequest to GameEventMsg
pub fn character_confirmation_request_to_game_event_msg(
    _event: acprotocol::gameevents::CharacterConfirmationRequest,
) -> GameEventMsg {
    GameEventMsg::CharacterConfirmationRequest {
        confirmation_type: 0,
        context: 0,
        message: String::new(),
    }
}

// Game Events Game Event Conversions

/// Convert acprotocol GameGameJoinGameResponse to GameEventMsg
pub fn game_join_game_response_to_game_event_msg(
    _event: acprotocol::gameevents::GameJoinGameResponse,
) -> GameEventMsg {
    GameEventMsg::GameJoinGameResponse {
        game_id: 0,
        team_id: 0,
    }
}

/// Convert acprotocol GameGameStartGame to GameEventMsg
pub fn game_start_game_to_game_event_msg(
    _event: acprotocol::gameevents::GameStartGame,
) -> GameEventMsg {
    GameEventMsg::GameStartGame { game_id: 0 }
}

/// Convert acprotocol GameGameMoveResponse to GameEventMsg
pub fn game_move_response_to_game_event_msg(
    _event: acprotocol::gameevents::GameMoveResponse,
) -> GameEventMsg {
    GameEventMsg::GameMoveResponse {
        game_id: 0,
        move_data: vec![],
    }
}

/// Convert acprotocol GameGameOpponentTurn to GameEventMsg
pub fn game_opponent_turn_to_game_event_msg(
    _event: acprotocol::gameevents::GameOpponentTurn,
) -> GameEventMsg {
    GameEventMsg::GameOpponentTurn {
        game_id: 0,
        move_data: vec![],
    }
}

/// Convert acprotocol GameGameOpponentStalemateState to GameEventMsg
pub fn game_opponent_stalemate_state_to_game_event_msg(
    _event: acprotocol::gameevents::GameOpponentStalemateState,
) -> GameEventMsg {
    GameEventMsg::GameOpponentStalemateState { game_id: 0 }
}

/// Convert acprotocol GameGameGameOver to GameEventMsg
pub fn game_game_over_to_game_event_msg(
    _event: acprotocol::gameevents::GameGameOver,
) -> GameEventMsg {
    GameEventMsg::GameGameOver {
        game_id: 0,
        winner_id: 0,
    }
}

// Phase 4: Channels Game Event Conversions

/// Convert acprotocol CommunicationCommunicationChannelBroadcast to GameEventMsg
pub fn channel_broadcast_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationChannelBroadcast,
) -> GameEventMsg {
    GameEventMsg::CommunicationChannelBroadcast {
        channel_id: 0,
        sender_name: String::new(),
        message: String::new(),
    }
}

/// Convert acprotocol CommunicationCommunicationChannelList to GameEventMsg
pub fn channel_list_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationChannelList,
) -> GameEventMsg {
    GameEventMsg::CommunicationChannelList { channels: vec![] }
}

/// Convert acprotocol CommunicationChannelIndex to GameEventMsg
pub fn channel_index_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationChannelIndex,
) -> GameEventMsg {
    GameEventMsg::CommunicationChannelIndex {
        channel_id: 0,
        channel_name: String::new(),
    }
}

// Phase 5: Admin Game Event Conversions

/// Convert acprotocol AdminQueryPlugin to GameEventMsg
pub fn admin_query_plugin_to_game_event_msg(
    _event: acprotocol::gameevents::AdminQueryPlugin,
) -> GameEventMsg {
    GameEventMsg::AdminQueryPlugin {
        plugin_name: String::new(),
    }
}

/// Convert acprotocol AdminQueryPluginList to GameEventMsg
pub fn admin_query_plugin_list_to_game_event_msg(
    _event: acprotocol::gameevents::AdminQueryPluginList,
) -> GameEventMsg {
    GameEventMsg::AdminQueryPluginList { plugins: vec![] }
}

/// Convert acprotocol AdminQueryPluginResponse2 to GameEventMsg
pub fn admin_query_plugin_response_to_game_event_msg(
    _event: acprotocol::gameevents::AdminQueryPluginResponse2,
) -> GameEventMsg {
    GameEventMsg::AdminQueryPluginResponse {
        plugin_name: String::new(),
        plugin_data: vec![],
    }
}

// Phase 5: Portal Storm Game Event Conversions

/// Convert acprotocol MiscPortalStormBrewing to GameEventMsg
pub fn misc_portal_storm_brewing_to_game_event_msg(
    _event: acprotocol::gameevents::MiscPortalStormBrewing,
) -> GameEventMsg {
    GameEventMsg::MiscPortalStormBrewing {}
}

/// Convert acprotocol MiscPortalStormImminent to GameEventMsg
pub fn misc_portal_storm_imminent_to_game_event_msg(
    _event: acprotocol::gameevents::MiscPortalStormImminent,
) -> GameEventMsg {
    GameEventMsg::MiscPortalStormImminent {}
}

/// Convert acprotocol MiscPortalStorm to GameEventMsg
pub fn misc_portal_storm_to_game_event_msg(
    _event: acprotocol::gameevents::MiscPortalStorm,
) -> GameEventMsg {
    GameEventMsg::MiscPortalStorm {}
}

/// Convert acprotocol MiscPortalStormSubsided to GameEventMsg
pub fn misc_portal_storm_subsided_to_game_event_msg(
    _event: acprotocol::gameevents::MiscPortalStormSubsided,
) -> GameEventMsg {
    GameEventMsg::MiscPortalStormSubsided {}
}

// Phase 5: Additional Communication Game Event Conversions

/// Convert acprotocol CommunicationPopUpString to GameEventMsg
pub fn communication_popup_string_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationPopUpString,
) -> GameEventMsg {
    GameEventMsg::CommunicationPopUpString {
        message: String::new(),
    }
}

/// Convert acprotocol CommunicationWeenieError to GameEventMsg
pub fn communication_weenie_error_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationWeenieError,
) -> GameEventMsg {
    GameEventMsg::CommunicationWeenieError { error_code: 0 }
}

/// Convert acprotocol CommunicationWeenieErrorWithString to GameEventMsg
pub fn communication_weenie_error_with_string_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationWeenieErrorWithString,
) -> GameEventMsg {
    GameEventMsg::CommunicationWeenieErrorWithString {
        error_code: 0,
        message: String::new(),
    }
}

/// Convert acprotocol CommunicationSetSquelchDB to GameEventMsg
pub fn communication_set_squelch_db_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationSetSquelchDB,
) -> GameEventMsg {
    GameEventMsg::CommunicationSetSquelchDB {
        squelch_data: vec![],
    }
}

/// Convert acprotocol CommunicationChatRoomTracker to GameEventMsg
pub fn communication_chat_room_tracker_to_game_event_msg(
    _event: acprotocol::gameevents::CommunicationChatRoomTracker,
) -> GameEventMsg {
    GameEventMsg::CommunicationChatRoomTracker {
        chat_room_id: 0,
        chat_room_name: String::new(),
    }
}

// Salvage Game Event Conversions

/// Convert acprotocol InventorySalvageOperationsResultData to GameEventMsg
pub fn inventory_salvage_operations_result_to_game_event_msg(
    _event: acprotocol::gameevents::InventorySalvageOperationsResultData,
) -> GameEventMsg {
    GameEventMsg::InventorySalvageOperationsResultData {
        success: false,
        salvage_type: 0,
        amount: 0,
    }
}

// Login Game Event Conversions

/// Convert acprotocol LoginPlayerDescription to GameEventMsg
pub fn login_player_description_to_game_event_msg(
    _event: acprotocol::gameevents::LoginPlayerDescription,
) -> GameEventMsg {
    GameEventMsg::LoginPlayerDescription {
        player_id: 0,
        description: vec![],
    }
}

// Character Game Event Conversions

/// Convert acprotocol CharacterReturnPing to GameEventMsg
pub fn character_return_ping_to_game_event_msg(
    _event: acprotocol::gameevents::CharacterReturnPing,
) -> GameEventMsg {
    GameEventMsg::CharacterReturnPing { sequence: 0 }
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
