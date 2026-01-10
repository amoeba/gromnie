use acprotocol::readers::{ACDataType, ACReader};
use tokio::sync::mpsc;
use tracing::error;

use crate::client::{ClientEvent, GameEvent};
use gromnie_events::{OrderedGameEvent, ProtocolEvent};

/// Trait for handling a specific parsed game event type.
///
/// Implementers focus ONLY on business logic - parsing, error handling,
/// and event emission are handled by the dispatcher.
///
/// # Example
///
/// ```ignore
/// impl GameEventHandler<CommunicationHearDirectSpeech> for Client {
///     fn handle(&mut self, event: CommunicationHearDirectSpeech) -> Option<GameEvent> {
///         let text = format!("{} tells you, \"{}\"", event.sender_name, event.message);
///         Some(GameEvent::ChatMessageReceived {
///             message: text,
///             message_type: event.message_type,
///         })
///     }
/// }
/// ```
pub trait GameEventHandler<T: ACDataType> {
    /// Process the parsed game event and optionally return a GameEvent.
    ///
    /// Return None if no event should be emitted (e.g., internal state updates only
    /// or when the event is sent asynchronously).
    ///
    /// Mutate self for state updates as needed.
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed event data
    fn handle(&mut self, parsed: T) -> Option<GameEvent>;
}

/// Dispatch a game event: parse → emit protocol event → handle → emit game event.
///
/// Centralizes the repetitive pattern across all game event handlers:
/// 1. Parse the cursor data into the specific event type T
/// 2. Handle parse errors by logging (non-fatal)
/// 3. Emit the ProtocolEvent automatically (infrastructure)
/// 4. Call the handler's handle() method with the parsed data (business logic)
/// 5. Emit the resulting GameEvent to the event bus (if any)
///
/// # Type Parameters
///
/// - `T`: The parsed game event type (must implement ACDataType + Clone)
/// - `H`: The handler type (must implement GameEventHandler<T>)
///
/// # Arguments
///
/// - `handler`: The client or handler instance
/// - `cursor`: Cursor positioned after the event opcode
/// - `event_tx`: Channel to send events to
/// - `object_id`: The object ID from the OrderedGameEvent wrapper
/// - `sequence`: The sequence number from the OrderedGameEvent wrapper
/// - `to_game_event_msg`: Function to convert T into GameEventMsg
///
/// # Returns
///
/// Ok(()) if parsing and handling succeeded, Err with error message if parsing failed.
pub fn dispatch_game_event<T, H, F>(
    handler: &mut H,
    cursor: &mut dyn ACReader,
    event_tx: &mpsc::Sender<ClientEvent>,
    object_id: u32,
    sequence: u32,
    to_game_event_msg: F,
) -> Result<(), String>
where
    T: ACDataType + Clone,
    H: GameEventHandler<T>,
    F: FnOnce(T) -> gromnie_events::GameEventMsg,
{
    // Parse game event data
    let parsed = match T::read(cursor) {
        Ok(p) => p,
        Err(e) => {
            error!(target: "net", "Failed to parse game event: {}", e);
            return Err(format!("Parse error: {}", e));
        }
    };

    // Emit protocol event (infrastructure - happens automatically for all game events)
    let protocol_event = ProtocolEvent::GameEvent(OrderedGameEvent {
        object_id,
        sequence,
        event: to_game_event_msg(parsed.clone()),
    });
    let _ = event_tx.try_send(ClientEvent::Protocol(protocol_event));

    // Handle and optionally emit game event (business logic)
    if let Some(game_event) = handler.handle(parsed) {
        let _ = event_tx.try_send(ClientEvent::Game(game_event));
    }

    Ok(())
}

// Magic & Items GameEventHandler Implementations

use crate::client::Client;

impl GameEventHandler<acprotocol::gameevents::MagicUpdateEnchantment> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::MagicUpdateEnchantment,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::MagicRemoveEnchantment> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::MagicRemoveEnchantment,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::ItemSetAppraiseInfo> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::ItemSetAppraiseInfo) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::ItemAppraiseDone> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::ItemAppraiseDone) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::ItemWearItem> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::ItemWearItem) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::ItemQueryItemManaResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::ItemQueryItemManaResponse,
    ) -> Option<GameEvent> {
        None
    }
}

// Trade GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::TradeOpenTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeOpenTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeCloseTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeCloseTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeAddToTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeAddToTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeRemoveFromTrade> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::TradeRemoveFromTrade,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeAcceptTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeAcceptTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeDeclineTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeDeclineTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeResetTrade> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeResetTrade) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeTradeFailure> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::TradeTradeFailure) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::TradeClearTradeAcceptance> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::TradeClearTradeAcceptance,
    ) -> Option<GameEvent> {
        None
    }
}

// Fellowship GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::FellowshipUpdateFellow> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::FellowshipUpdateFellow,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipDisband> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipDisband) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipQuit> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipQuit) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipDismiss> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipDismiss) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipFellowUpdateDone> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::FellowshipFellowUpdateDone,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipFellowStatsDone> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::FellowshipFellowStatsDone,
    ) -> Option<GameEvent> {
        None
    }
}

// Social GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::SocialFriendsUpdate> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialFriendsUpdate) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialCharacterTitleTable> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::SocialCharacterTitleTable,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialAddOrSetCharacterTitle> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::SocialAddOrSetCharacterTitle,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialSendClientContractTrackerTable> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::SocialSendClientContractTrackerTable,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialSendClientContractTracker> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::SocialSendClientContractTracker,
    ) -> Option<GameEvent> {
        None
    }
}

// Allegiance GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdate> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AllegianceAllegianceUpdate,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdateDone> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AllegianceAllegianceUpdateDone,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdateAborted> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AllegianceAllegianceUpdateAborted,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceLoginNotificationEvent>
    for Client
{
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AllegianceAllegianceLoginNotificationEvent,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceInfoResponseEvent> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AllegianceAllegianceInfoResponseEvent,
    ) -> Option<GameEvent> {
        None
    }
}

// Vendor GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::VendorVendorInfo> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::VendorVendorInfo) -> Option<GameEvent> {
        None
    }
}

// Housing GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::HouseHouseProfile> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::HouseHouseProfile) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseHouseData> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::HouseHouseData) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseHouseStatus> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::HouseHouseStatus) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseUpdateRentTime> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::HouseUpdateRentTime) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseUpdateRentPayment> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::HouseUpdateRentPayment,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseUpdateRestrictions> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::HouseUpdateRestrictions,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseUpdateHAR> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::HouseUpdateHAR) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseHouseTransaction> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::HouseHouseTransaction,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::HouseAvailableHouses> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::HouseAvailableHouses,
    ) -> Option<GameEvent> {
        None
    }
}

// Writing GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::WritingBookOpen> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::WritingBookOpen) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::WritingBookAddPageResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::WritingBookAddPageResponse,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::WritingBookDeletePageResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::WritingBookDeletePageResponse,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::WritingBookPageDataResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::WritingBookPageDataResponse,
    ) -> Option<GameEvent> {
        None
    }
}

// Character GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::CharacterStartBarber> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CharacterStartBarber,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CharacterQueryAgeResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CharacterQueryAgeResponse,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CharacterConfirmationRequest> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CharacterConfirmationRequest,
    ) -> Option<GameEvent> {
        None
    }
}

// Game Events GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::GameJoinGameResponse> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::GameJoinGameResponse,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::GameStartGame> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::GameStartGame) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::GameMoveResponse> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::GameMoveResponse) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::GameOpponentTurn> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::GameOpponentTurn) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::GameOpponentStalemateState> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::GameOpponentStalemateState,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::GameGameOver> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::GameGameOver) -> Option<GameEvent> {
        None
    }
}

// Channels GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::CommunicationChannelBroadcast> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationChannelBroadcast,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationChannelList> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationChannelList,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationChannelIndex> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationChannelIndex,
    ) -> Option<GameEvent> {
        None
    }
}

// Admin GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::AdminQueryPlugin> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AdminQueryPlugin) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AdminQueryPluginList> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AdminQueryPluginList,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AdminQueryPluginResponse2> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::AdminQueryPluginResponse2,
    ) -> Option<GameEvent> {
        None
    }
}

// Portal Storms GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::MiscPortalStormBrewing> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::MiscPortalStormBrewing,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::MiscPortalStormImminent> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::MiscPortalStormImminent,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::MiscPortalStorm> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::MiscPortalStorm) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::MiscPortalStormSubsided> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::MiscPortalStormSubsided,
    ) -> Option<GameEvent> {
        None
    }
}

// Additional Communication GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::CommunicationPopUpString> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationPopUpString,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationWeenieError> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationWeenieError,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationWeenieErrorWithString> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationWeenieErrorWithString,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationSetSquelchDB> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationSetSquelchDB,
    ) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::CommunicationChatRoomTracker> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::CommunicationChatRoomTracker,
    ) -> Option<GameEvent> {
        None
    }
}

// Salvage GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::InventorySalvageOperationsResultData> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::InventorySalvageOperationsResultData,
    ) -> Option<GameEvent> {
        None
    }
}

// Login GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::LoginPlayerDescription> for Client {
    fn handle(
        &mut self,
        _event: acprotocol::gameevents::LoginPlayerDescription,
    ) -> Option<GameEvent> {
        None
    }
}

// Character GameEventHandler Implementations

impl GameEventHandler<acprotocol::gameevents::CharacterReturnPing> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::CharacterReturnPing) -> Option<GameEvent> {
        None
    }
}
