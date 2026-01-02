use acprotocol::readers::{ACDataType, ACReader};
use tokio::sync::mpsc;
use tracing::error;

use crate::client::{ClientEvent, GameEvent};

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
///             message_type: event.type_ as u32,
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
    fn handle(&mut self, parsed: T) -> Option<GameEvent>;
}

/// Dispatch a game event: parse → handle → emit event.
///
/// Centralizes the repetitive pattern across all game event handlers:
/// 1. Parse the cursor data into the specific event type T
/// 2. Handle parse errors by logging (non-fatal)
/// 3. Call the handler's handle() method with the parsed data
/// 4. Emit the resulting GameEvent to the event bus (if any)
///
/// # Type Parameters
///
/// - `T`: The parsed game event type (must implement ACDataType)
/// - `H`: The handler type (must implement GameEventHandler<T>)
///
/// # Arguments
///
/// - `handler`: The client or handler instance
/// - `cursor`: Cursor positioned after the event opcode
/// - `event_tx`: Channel to send events to
///
/// # Returns
///
/// Ok(()) if parsing and handling succeeded, Err with error message if parsing failed.
pub fn dispatch_game_event<T, H>(
    handler: &mut H,
    cursor: &mut dyn ACReader,
    event_tx: &mpsc::Sender<ClientEvent>,
) -> Result<(), String>
where
    T: ACDataType,
    H: GameEventHandler<T>,
{
    // Parse game event data
    let parsed = match T::read(cursor) {
        Ok(p) => p,
        Err(e) => {
            error!(target: "net", "Failed to parse game event: {}", e);
            return Err(format!("Parse error: {}", e));
        }
    };

    // Handle and optionally emit event
    if let Some(game_event) = handler.handle(parsed) {
        let _ = event_tx.try_send(ClientEvent::Game(game_event));
    }

    Ok(())
}
