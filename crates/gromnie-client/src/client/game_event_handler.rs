use asheron_rs::readers::{ACDataType, ACReader};
use tokio::sync::mpsc;
use tracing::error;

use gromnie_events::{OrderedGameEvent, ProtocolEvent};

use crate::client::ClientEvent;

/// Trait for handling a specific parsed game event type.
///
/// Implementers focus ONLY on business logic. The protocol event is emitted
/// automatically by [`dispatch_game_event`] before `handle` is called, so
/// implementers only need to react to the event, not re-publish it.
pub trait GameEventHandler<T: ACDataType> {
    /// Process the parsed game event.
    ///
    /// Mutate self for state updates as needed. The corresponding
    /// [`ClientEvent::Protocol`] event has already been emitted by the
    /// dispatcher before this runs.
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed event data
    fn handle(&mut self, parsed: T);
}

/// Dispatch a game event: parse → emit protocol event → handle.
///
/// Centralizes the repetitive pattern across all game event handlers:
/// 1. Parse the cursor data into the specific event type T
/// 2. Handle parse errors by logging (non-fatal)
/// 3. Emit the ProtocolEvent automatically (infrastructure)
/// 4. Call the handler's handle() method with the parsed data (business logic)
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

    handler.handle(parsed);

    Ok(())
}
