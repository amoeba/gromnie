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

// ============================================================================
// Phase 2: Magic & Items GameEventHandler Implementations
// ============================================================================

use crate::client::Client;

impl GameEventHandler<acprotocol::gameevents::MagicUpdateEnchantment> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::MagicUpdateEnchantment) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::MagicRemoveEnchantment> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::MagicRemoveEnchantment) -> Option<GameEvent> {
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
    fn handle(&mut self, _event: acprotocol::gameevents::ItemQueryItemManaResponse) -> Option<GameEvent> {
        None
    }
}

// ============================================================================
// Phase 3: Trade GameEventHandler Implementations
// ============================================================================

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
    fn handle(&mut self, _event: acprotocol::gameevents::TradeRemoveFromTrade) -> Option<GameEvent> {
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
    fn handle(&mut self, _event: acprotocol::gameevents::TradeClearTradeAcceptance) -> Option<GameEvent> {
        None
    }
}

// ============================================================================
// Phase 3: Fellowship GameEventHandler Implementations
// ============================================================================

impl GameEventHandler<acprotocol::gameevents::FellowshipUpdateFellow> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipUpdateFellow) -> Option<GameEvent> {
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
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipFellowUpdateDone) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::FellowshipFellowStatsDone> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::FellowshipFellowStatsDone) -> Option<GameEvent> {
        None
    }
}

// ============================================================================
// Phase 3: Social GameEventHandler Implementations
// ============================================================================

impl GameEventHandler<acprotocol::gameevents::SocialFriendsUpdate> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialFriendsUpdate) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialCharacterTitleTable> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialCharacterTitleTable) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialAddOrSetCharacterTitle> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialAddOrSetCharacterTitle) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialSendClientContractTrackerTable> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialSendClientContractTrackerTable) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::SocialSendClientContractTracker> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::SocialSendClientContractTracker) -> Option<GameEvent> {
        None
    }
}

// ============================================================================
// Phase 3: Allegiance GameEventHandler Implementations
// ============================================================================

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdate> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AllegianceAllegianceUpdate) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdateDone> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AllegianceAllegianceUpdateDone) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceUpdateAborted> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AllegianceAllegianceUpdateAborted) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceLoginNotificationEvent> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AllegianceAllegianceLoginNotificationEvent) -> Option<GameEvent> {
        None
    }
}

impl GameEventHandler<acprotocol::gameevents::AllegianceAllegianceInfoResponseEvent> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::AllegianceAllegianceInfoResponseEvent) -> Option<GameEvent> {
        None
    }
}

// ============================================================================
// Phase 3: Vendor GameEventHandler Implementations
// ============================================================================

impl GameEventHandler<acprotocol::gameevents::VendorVendorInfo> for Client {
    fn handle(&mut self, _event: acprotocol::gameevents::VendorVendorInfo) -> Option<GameEvent> {
        None
    }
}
