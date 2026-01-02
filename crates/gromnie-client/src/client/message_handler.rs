use std::io::Cursor;
use acprotocol::network::RawMessage;
use acprotocol::readers::ACDataType;
use tokio::sync::mpsc;
use tracing::error;

use crate::client::events::{ClientEvent, GameEvent};

/// Trait for handling a specific parsed message type.
///
/// Implementers focus ONLY on business logic - parsing, error handling,
/// and event emission are handled by the dispatcher.
///
/// # Example
///
/// ```ignore
/// impl MessageHandler<CommunicationHearRangedSpeech> for Client {
///     fn handle(&mut self, speech: CommunicationHearRangedSpeech) -> Option<GameEvent> {
///         let text = format!("{} says, \"{}\"", speech.sender_name, speech.message);
///         Some(GameEvent::ChatMessageReceived {
///             message: text,
///             message_type: speech.type_ as u32,
///         })
///     }
/// }
/// ```
pub trait MessageHandler<T: ACDataType> {
    /// Process the parsed message and optionally return a GameEvent.
    ///
    /// Return None if no event should be emitted (e.g., internal state updates only
    /// or when the event is sent asynchronously).
    ///
    /// Mutate self for state updates as needed.
    fn handle(&mut self, parsed: T) -> Option<GameEvent>;
}

/// Dispatch a message: parse → handle → emit event.
///
/// Centralizes the repetitive pattern across all message handlers:
/// 1. Parse the RawMessage into the specific message type T
/// 2. Handle parse errors by logging (non-fatal)
/// 3. Call the handler's handle() method with the parsed data
/// 4. Emit the resulting GameEvent to the event bus (if any)
///
/// # Type Parameters
///
/// - `T`: The parsed message type (must implement ACDataType)
/// - `H`: The handler type (must implement MessageHandler<T>)
///
/// # Arguments
///
/// - `handler`: The client or handler instance
/// - `message`: The raw message from the network
/// - `event_tx`: Channel to send events to
///
/// # Returns
///
/// Ok(()) if parsing and handling succeeded, Err with error message if parsing failed.
pub fn dispatch_message<T, H>(
    handler: &mut H,
    message: RawMessage,
    event_tx: &mpsc::Sender<ClientEvent>,
) -> Result<(), String>
where
    T: ACDataType,
    H: MessageHandler<T>,
{
    // Parse message (skip 4-byte opcode prefix)
    let mut cursor = Cursor::new(&message.data[4..]);
    let parsed = match T::read(&mut cursor) {
        Ok(p) => p,
        Err(e) => {
            error!(target: "net", "Failed to parse message: {}", e);
            return Err(format!("Parse error: {}", e));
        }
    };

    // Handle and optionally emit event
    if let Some(game_event) = handler.handle(parsed) {
        let _ = event_tx.try_send(ClientEvent::Game(game_event));
    }

    Ok(())
}
