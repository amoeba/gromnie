use asheron_rs::network::RawMessage;
use asheron_rs::readers::ACDataType;
use std::io::Cursor;
use tracing::error;

/// Trait for handling a specific parsed message type.
///
/// Implementers focus on business logic only. Each handler is responsible for
/// emitting its own `ClientEvent::Protocol` event, since only the handler knows
/// how to convert its message into the strongly-typed protocol representation.
///
/// # Example
///
/// ```ignore
/// impl MessageHandler<CommunicationHearRangedSpeech> for Client {
///     fn handle(&mut self, speech: CommunicationHearRangedSpeech) {
///         self.emit_protocol(speech.to_protocol_event());
///     }
/// }
/// ```
pub trait MessageHandler<T: ACDataType> {
    /// Process the parsed message.
    ///
    /// Mutate self for state updates as needed, and emit any events this
    /// message implies.
    fn handle(&mut self, parsed: T);
}

/// Dispatch a message: parse → handle.
///
/// Centralizes the repetitive pattern across all message handlers:
/// 1. Parse the RawMessage into the specific message type T
/// 2. Handle parse errors by logging (non-fatal)
/// 3. Call the handler's handle() method with the parsed data
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
///
/// # Returns
///
/// Ok(()) if parsing and handling succeeded, Err with error message if parsing failed.
pub fn dispatch_message<T, H>(handler: &mut H, message: RawMessage) -> Result<(), String>
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

    handler.handle(parsed);

    Ok(())
}
