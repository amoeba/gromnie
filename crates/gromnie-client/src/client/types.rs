use crate::client::messages::OutgoingMessageContent;

// Note: CharacterInfo is now defined in gromnie-events and re-exported

/// Internal client actions (internal-only variants not exposed via SimpleClientAction)
#[derive(Debug)]
pub enum ClientAction {
    /// Send a raw message to the server (internal only)
    SendMessage(Box<OutgoingMessageContent>),
}
