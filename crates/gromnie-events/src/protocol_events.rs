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
    // ===== Quality/Property Updates =====
    QualitiesPrivateUpdateInt {
        sequence: u8,
        property: String,
        value: i32,
    },
    QualitiesUpdateInt {
        sequence: u8,
        object_id: u32,
        property: String,
        value: i32,
    },
    QualitiesPrivateUpdateInt64 {
        sequence: u8,
        property: String,
        value: i64,
    },
    QualitiesUpdateInt64 {
        sequence: u8,
        object_id: u32,
        property: String,
        value: i64,
    },
    QualitiesPrivateUpdateBool {
        sequence: u8,
        property: String,
        value: bool,
    },
    QualitiesUpdateBool {
        sequence: u8,
        object_id: u32,
        property: String,
        value: bool,
    },
    QualitiesPrivateUpdateFloat {
        sequence: u8,
        property: String,
        value: f64,
    },
    QualitiesUpdateFloat {
        sequence: u8,
        object_id: u32,
        property: String,
        value: f64,
    },
    QualitiesPrivateUpdateString {
        sequence: u8,
        property: String,
        value: String,
    },
    QualitiesUpdateString {
        sequence: u8,
        object_id: u32,
        property: String,
        value: String,
    },
    QualitiesPrivateUpdateDataId {
        sequence: u8,
        property: String,
        value: u32,
    },
    QualitiesUpdateDataId {
        sequence: u8,
        object_id: u32,
        property: String,
        value: u32,
    },
    QualitiesPrivateUpdateInstanceId {
        sequence: u8,
        property: String,
        value: u32,
    },
    QualitiesUpdateInstanceId {
        sequence: u8,
        object_id: u32,
        property: String,
        value: u32,
    },
    QualitiesPrivateUpdatePosition {
        sequence: u8,
        object_id: u32,
        property: String,
        value: Vec<u8>, // Serialized Position data
    },
    QualitiesUpdatePosition {
        sequence: u8,
        object_id: u32,
        property: String,
        value: Vec<u8>, // Serialized Position data
    },
    // ===== Movement Messages =====
    MovementPositionAndMovement {
        object_id: u32,
        position: String, // JSON-serialized PositionPack
        movement_data: String, // JSON-serialized MovementData
    },
    MovementPosition {
        object_id: u32,
        position: String, // JSON-serialized PositionPack
    },
    MovementSetObjectMovement {
        object_id: u32,
        instance_sequence: u16,
        movement_data: String, // JSON-serialized MovementData
    },
    MovementVectorUpdate {
        object_id: u32,
        velocity_x: f32,
        velocity_y: f32,
        velocity_z: f32,
        omega_x: f32,
        omega_y: f32,
        omega_z: f32,
        instance_sequence: u16,
        vector_sequence: u16,
    },
    // ===== Item/Inventory Messages =====
    ItemUpdateObject {
        object_id: u32,
        model_data: Vec<u8>, // Serialized model/object data
    },
    ItemParent {
        object_id: u32,
        parent_id: u32,
        placement: u32,
    },
    InventoryPickup {
        object_id: u32,
    },
    ItemServerSaysRemove {
        object_id: u32,
    },
    ItemUpdateStackSize {
        object_id: u32,
        stack_size: i32,
    },
    ItemObjDesc {
        object_id: u32,
        obj_desc: Vec<u8>, // Serialized ObjDesc data
    },
    // ===== Communication Messages =====
    CommunicationHearEmote {
        sender_name: String,
        message: String,
    },
    CommunicationHearSoulEmote {
        sender_name: String,
        message: String,
    },
    CommunicationTurbineChat {
        message_type: u32,
        channel_id: u32,
        sender_name: String,
        message: String,
    },
    CommunicationTextboxString {
        message: String,
        message_type: u32,
    },
    // ===== Visual/Effects Messages =====
    EffectsSoundEvent {
        object_id: u32,
        sound_id: u32,
    },
    EffectsPlayerTeleport {
        position: Vec<u8>, // Serialized position
    },
    EffectsPlayScriptId {
        object_id: u32,
        script_id: u32,
    },
    EffectsPlayScriptType {
        object_id: u32,
        script_type: u32,
    },
    CharacterSetPlayerVisualDesc {
        object_id: u32,
        visual_desc: Vec<u8>, // Serialized visual description
    },
    // ===== Login/Character Messages =====
    LoginWorldInfo {
        connections: u32,
        max_connections: u32,
        world_name: String,
    },
    LoginAccountBanned {
        reason: String,
    },
    LoginAccountBooted {
        reason: String,
    },
    CharacterDelete {
        character_id: u32,
    },
    CharacterServerSaysAttemptFailed {
        failure_type: u32,
    },
    // ===== Admin Messages =====
    AdminReceivePlayerData {
        player_id: u32,
        data: Vec<u8>,
    },
    AdminReceiveAccountData {
        account_name: String,
        data: Vec<u8>,
    },
    // ===== Combat/Death Messages =====
    CombatHandlePlayerDeath {
        victim_id: u32,
        killer_id: u32,
    },
    // ===== Phase 2: Magic & Items Messages =====
    // Magic/Enchantment Messages
    MagicUpdateEnchantmentS2C {
        enchantment_id: u32,
        spell_id: u32,
        layer: u32,
        caster_id: u32,
        duration: f32,
    },
    MagicRemoveEnchantmentS2C {
        enchantment_id: u32,
        layer: u32,
    },
    MagicEnchantmentAlreadyPresent {
        enchantment_id: u32,
        layer: u32,
    },
    MagicEnchantmentRemovalFailed {
        reason_code: u32,
    },
    // Item Appraisal & Properties
    ItemAppriseInfo {
        object_id: u32,
        appraisal_data: Vec<u8>,
    },
    ItemAppriseInfoDone {
        object_id: u32,
        success: bool,
    },
    // Equipment Messages
    ItemWearOutfit {
        object_id: u32,
        placer_id: u32,
        slot_index: u32,
    },
    ItemUnwearOutfit {
        object_id: u32,
        placer_id: u32,
    },
    // Container/Inventory Messages
    ItemContainersViewData {
        container_id: u32,
        contents: Vec<u32>,
    },
    ItemContainerIdUpdate {
        object_id: u32,
        container_id: u32,
    },
    ItemMoveItemRequest {
        object_id: u32,
        source_container: u32,
        destination_container: u32,
        placement: u32,
    },
    ItemMoveItemResponse {
        success: bool,
        object_id: u32,
        reason_code: u32,
    },
    ItemEncumbranceUpdate {
        current_encumbrance: u32,
        max_encumbrance: u32,
    },
    // Item Query Responses
    ItemQueryItemManaResponseS2C {
        object_id: u32,
        current_mana: u32,
        max_mana: u32,
    },
    ItemGetInscriptionResponseS2C {
        object_id: u32,
        inscription: String,
        author_id: u32,
        author_name: String,
        author_account: String,
    },
    ItemQueryItemSchoolsResponseS2C {
        object_id: u32,
        schools: Vec<u32>,
    },
    ItemQueryItemVendorResponse {
        object_id: u32,
        vendor_id: u32,
        vendor_name: String,
        vendor_price: u32,
    },
    // ===== Phase 3: Social Systems Messages =====
    // Trade System
    TradeRegisterTrade {
        initiator_id: u32,
        partner_id: u32,
    },
    TradeOpenTrade {
        partner_id: u32,
        partner_name: String,
    },
    TradeCloseTrade {
        reason: u32,
    },
    TradeAddToTrade {
        item_id: u32,
        trader_id: u32,
    },
    TradeRemoveFromTrade {
        item_id: u32,
        trader_id: u32,
    },
    TradeAcceptTrade {
        trader_id: u32,
    },
    TradeDeclineTrade {},
    TradeResetTrade {},
    TradeTradeFailure {
        reason: u32,
    },
    TradeClearTradeAcceptance {},
    // Fellowship System
    FellowshipFullUpdate {
        fellowship_data: Vec<u8>,
    },
    FellowshipUpdateFellow {
        fellow_id: u32,
        fellow_data: Vec<u8>,
    },
    FellowshipUpdateDone {},
    FellowshipDisband {},
    FellowshipQuit {
        fellow_id: u32,
    },
    FellowshipDismiss {
        fellow_id: u32,
    },
    // Social Features
    FriendsUpdate {
        friends: Vec<(u32, String, bool)>,
    },
    CharacterTitleTable {
        titles: Vec<(u32, String)>,
    },
    AddOrSetCharacterTitle {
        title_id: u32,
        title: String,
        active: bool,
    },
    // Contracts
    SendClientContractTrackerTable {
        contracts: Vec<u32>,
    },
    SendClientContractTracker {
        contract_id: u32,
        stage: u32,
        timestamp: u64,
    },
    // Allegiance
    AllegianceUpdate {
        allegiance_data: Vec<u8>,
    },
    AllegianceUpdateDone {},
    AllegianceUpdateAborted {},
    AllegianceLoginNotification {
        member_name: String,
        member_id: u32,
    },
    AllegianceInfoResponse {
        allegiance_name: String,
        allegiance_data: Vec<u8>,
    },
    // Vendor
    VendorInfo {
        vendor_id: u32,
        vendor_type: u32,
        items: Vec<u32>,
    },
    // ===== Phase 4: Advanced Features Messages =====
    // Housing
    HouseProfile {
        house_id: u32,
        owner_id: u32,
        house_type: u32,
    },
    HouseData {
        house_id: u32,
        position: Vec<u8>,
    },
    HouseStatus {
        house_id: u32,
        status: u32,
    },
    HouseUpdateRentTime {
        house_id: u32,
        rent_time: u32,
    },
    HouseUpdateRentPayment {
        house_id: u32,
        payment: u32,
    },
    HouseUpdateRestrictions {
        house_id: u32,
        restrictions: u32,
    },
    HouseUpdateHAR {
        house_id: u32,
        har: Vec<u8>,
    },
    HouseTransaction {
        house_id: u32,
        transaction_type: u32,
    },
    HouseAvailableHouses {
        houses: Vec<u32>,
    },
    // Writing/Books
    WritingBookOpen {
        book_id: u32,
        pages: u32,
    },
    WritingBookAddPageResponse {
        book_id: u32,
        success: bool,
    },
    WritingBookDeletePageResponse {
        book_id: u32,
        success: bool,
    },
    WritingBookPageDataResponse {
        book_id: u32,
        page: u32,
        content: String,
    },
    // Character Customization
    CharacterStartBarber {
        barber_id: u32,
    },
    CharacterQueryAgeResponse {
        age: u32,
    },
    CharacterConfirmationRequest {
        confirmation_type: u32,
        context: u32,
        message: String,
    },
    // Games
    GameJoinGameResponse {
        game_id: u32,
        team_id: u32,
    },
    GameStartGame {
        game_id: u32,
    },
    GameMoveResponse {
        game_id: u32,
        move_data: Vec<u8>,
    },
    GameOpponentTurn {
        game_id: u32,
        move_data: Vec<u8>,
    },
    GameOpponentStalemateState {
        game_id: u32,
    },
    GameGameOver {
        game_id: u32,
        winner_id: u32,
    },
    // Channels
    ChannelBroadcast {
        channel_id: u32,
        sender_name: String,
        message: String,
    },
    ChannelList {
        channels: Vec<(u32, String)>,
    },
    ChannelIndex {
        channel_id: u32,
        channel_name: String,
    },
    // ===== Phase 5: Polish Messages =====
    // Admin Tools
    ReceivePlayerData {
        player_id: u32,
        data: Vec<u8>,
    },
    QueryPlugin {
        plugin_name: String,
    },
    QueryPluginList {
        plugins: Vec<String>,
    },
    QueryPluginResponse {
        plugin_name: String,
        plugin_data: Vec<u8>,
    },
    // Advanced Communication
    TurbineChat {
        message_type: u32,
        channel_id: u32,
        sender_name: String,
        message: String,
    },
    TextboxString {
        message: String,
        message_type: u32,
    },
    PopUpString {
        message: String,
    },
    WeenieError {
        error_code: u32,
    },
    WeenieErrorWithString {
        error_code: u32,
        message: String,
    },
    // Portal Storms
    PortalStormBrewing {},
    PortalStormImminent {},
    PortalStorm {},
    PortalStormSubsided {},
    // Salvage
    SalvageOperationsResultData {
        success: bool,
        salvage_type: u32,
        amount: u32,
    },
    // Misc
    LoginPlayerDescription {
        player_id: u32,
        description: Vec<u8>,
    },
    ReturnPing {
        sequence: u32,
    },
    SetSquelchDB {
        squelch_data: Vec<u8>,
    },
    ChatRoomTracker {
        chat_room_id: u32,
        chat_room_name: String,
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
    // ===== Combat GameEvents =====
    CombatHandleAttackDone {
        target_id: u32,
    },
    CombatHandleVictimNotificationSelf {
        message: String,
        damage_type: u32,
        damage_amount: i32,
        critical: bool,
    },
    CombatHandleVictimNotificationOther {
        attacker_name: String,
        part_index: u32,
    },
    CombatHandleAttackerNotification {
        victim_name: String,
        damage_type: u32,
        damage_amount: f32,
        critical: bool,
        hit_location: u32,
    },
    CombatHandleDefenderNotification {
        attacker_name: String,
        damage_type: u32,
        evasion_result: u32,
    },
    CombatHandleEvasionAttackerNotification {
        defender_name: String,
        evasion_result: u32,
    },
    CombatHandleEvasionDefenderNotification {
        attacker_name: String,
        evasion_result: u32,
    },
    CombatHandleCommenceAttack {
        target_id: u32,
    },
    CombatQueryHealthResponse {
        target_id: u32,
        health_percent: f32,
    },
    CombatHandlePlayerDeath {
        message: String,
    },
    // ===== Item/Container GameEvents =====
    ItemOnViewContents {
        container_id: u32,
        items: Vec<u32>,
    },
    ItemServerSaysContainId {
        item_id: u32,
        container_id: u32,
    },
    ItemWearItem {
        object_id: u32,
        equipped_slot: u32,
    },
    ItemSetAppraiseInfo {
        object_id: u32,
        success: bool,
        appraisal_data: Vec<u8>, // Serialized appraisal data
    },
    ItemUseDone {
        error_code: u32,
    },
    ItemAppraiseDone {
        object_id: u32,
    },
    ItemStopViewingObjectContents {
        container_id: u32,
    },
    ItemServerSaysMoveItem {
        item_id: u32,
        from_container: u32,
        to_container: u32,
        placement: u32,
    },
    ItemQueryItemManaResponse {
        object_id: u32,
        mana: f32,
        max_mana: u32,
    },
    ItemGetInscriptionResponse {
        object_id: u32,
        inscription: String,
        author_name: String,
        author_account: String,
        author_id: u32,
    },
    // ===== Magic GameEvents =====
    MagicUpdateSpell {
        spell_id: u32,
    },
    MagicUpdateEnchantment {
        enchantment_id: u16,
        spell_id: u32,
        layer: u16,
        caster_id: u32,
        power_level: f32,
        start_time: f64,
        duration: f64,
    },
    MagicRemoveEnchantment {
        enchantment_id: u16,
    },
    MagicUpdateMultipleEnchantments {
        enchantments: Vec<u32>, // Simplified for now
    },
    MagicRemoveMultipleEnchantments {
        enchantments: Vec<u16>,
    },
    MagicPurgeEnchantments {
        caster_id: u32,
    },
    MagicDispelEnchantment {
        enchantment_id: u16,
        caster_id: u32,
    },
    MagicDispelMultipleEnchantments {
        enchantments: Vec<u16>,
    },
    MagicRemoveSpell {
        spell_id: u32,
    },
    MagicPurgeBadEnchantments {},
    // ===== Fellowship GameEvents =====
    FellowshipFullUpdate {
        fellowship_id: u32,
        leader_name: String,
        leader_id: u32,
        members: Vec<u32>,
        locked: bool,
    },
    FellowshipUpdateFellow {
        fellow_id: u32,
        fellow_name: String,
        health_percent: f32,
        stamina_percent: f32,
        mana_percent: f32,
        level: u32,
    },
    FellowshipDisband {},
    FellowshipQuit {
        fellow_id: u32,
    },
    FellowshipDismiss {
        fellow_id: u32,
    },
    FellowshipUpdateDone {},
    FellowshipStatsDone {},
    // ===== Social GameEvents =====
    SocialFriendsUpdate {
        friends: Vec<(u32, String, bool)>, // (id, name, online)
    },
    SocialCharacterTitleTable {
        titles: Vec<(u32, String)>, // (id, title)
    },
    SocialAddOrSetCharacterTitle {
        title_id: u32,
        title: String,
        active: bool,
    },
    SocialSendClientContractTrackerTable {
        contracts: Vec<u32>,
    },
    SocialSendClientContractTracker {
        contract_id: u32,
        stage: u32,
        timestamp: u64,
    },
    // ===== Trade GameEvents =====
    TradeRegisterTrade {
        initiator_id: u32,
        partner_id: u32,
    },
    TradeOpenTrade {
        partner_id: u32,
        partner_name: String,
    },
    TradeCloseTrade {
        reason: u32,
    },
    TradeAddToTrade {
        item_id: u32,
        trader_id: u32,
    },
    TradeRemoveFromTrade {
        item_id: u32,
        trader_id: u32,
    },
    TradeAcceptTrade {
        trader_id: u32,
    },
    TradeDeclineTrade {},
    TradeResetTrade {},
    TradeTradeFailure {
        reason: u32,
    },
    TradeClearTradeAcceptance {},
    // ===== Allegiance GameEvents =====
    AllegianceUpdate {
        allegiance_data: Vec<u8>, // Serialized allegiance data
    },
    AllegianceUpdateDone {},
    AllegianceUpdateAborted {},
    AllegianceLoginNotification {
        member_name: String,
        member_id: u32,
    },
    AllegianceInfoResponse {
        allegiance_name: String,
        allegiance_data: Vec<u8>,
    },
    // ===== Vendor GameEvents =====
    VendorInfo {
        vendor_id: u32,
        vendor_type: u32,
        items: Vec<u32>,
    },
    // ===== House GameEvents =====
    HouseProfile {
        house_id: u32,
        owner_id: u32,
        house_type: u32,
    },
    HouseData {
        house_id: u32,
        position: Vec<u8>,
    },
    HouseStatus {
        house_id: u32,
        status: u32,
    },
    HouseUpdateRentTime {
        house_id: u32,
        rent_time: u32,
    },
    HouseUpdateRentPayment {
        house_id: u32,
        payment: u32,
    },
    HouseUpdateRestrictions {
        house_id: u32,
        restrictions: u32,
    },
    HouseUpdateHAR {
        house_id: u32,
        har: Vec<u8>,
    },
    HouseTransaction {
        house_id: u32,
        transaction_type: u32,
    },
    HouseAvailableHouses {
        houses: Vec<u32>,
    },
    // ===== Writing GameEvents =====
    WritingBookOpen {
        book_id: u32,
        pages: u32,
    },
    WritingBookAddPageResponse {
        book_id: u32,
        success: bool,
    },
    WritingBookDeletePageResponse {
        book_id: u32,
        success: bool,
    },
    WritingBookPageDataResponse {
        book_id: u32,
        page: u32,
        content: String,
    },
    // ===== Character GameEvents =====
    CharacterStartBarber {
        barber_id: u32,
    },
    CharacterQueryAgeResponse {
        age: u32,
    },
    CharacterReturnPing {
        sequence: u32,
    },
    CharacterConfirmationRequest {
        confirmation_type: u32,
        context: u32,
        message: String,
    },
    CharacterConfirmationDone {
        confirmation_type: u32,
        context: u32,
        accepted: bool,
    },
    // ===== Communication GameEvents =====
    CommunicationPopUpString {
        message: String,
    },
    CommunicationChannelBroadcast {
        channel_id: u32,
        sender_name: String,
        message: String,
    },
    CommunicationChannelList {
        channels: Vec<(u32, String)>,
    },
    CommunicationChannelIndex {
        channel_id: u32,
        channel_name: String,
    },
    CommunicationSetSquelchDB {
        squelch_data: Vec<u8>,
    },
    CommunicationWeenieError {
        error_code: u32,
    },
    CommunicationWeenieErrorWithString {
        error_code: u32,
        message: String,
    },
    CommunicationChatRoomTracker {
        chat_room_id: u32,
        chat_room_name: String,
    },
    // ===== Game Events (Chess, etc.) =====
    GameJoinGameResponse {
        game_id: u32,
        team_id: u32,
    },
    GameStartGame {
        game_id: u32,
    },
    GameMoveResponse {
        game_id: u32,
        move_data: Vec<u8>,
    },
    GameOpponentTurn {
        game_id: u32,
        move_data: Vec<u8>,
    },
    GameOpponentStalemateState {
        game_id: u32,
    },
    GameGameOver {
        game_id: u32,
        winner_id: u32,
    },
    // ===== Admin GameEvents =====
    AdminQueryPluginList {
        plugins: Vec<String>,
    },
    AdminQueryPlugin {
        plugin_name: String,
    },
    AdminQueryPluginResponse {
        plugin_name: String,
        plugin_data: Vec<u8>,
    },
    // ===== Inventory GameEvents =====
    InventorySalvageOperationsResultData {
        success: bool,
        salvage_type: u32,
        amount: u32,
    },
    // ===== Login GameEvents =====
    LoginPlayerDescription {
        player_id: u32,
        description: Vec<u8>, // Serialized player description
    },
    // ===== Misc GameEvents =====
    MiscPortalStormBrewing {},
    MiscPortalStormImminent {},
    MiscPortalStorm {},
    MiscPortalStormSubsided {},
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
