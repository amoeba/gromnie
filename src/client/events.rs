use crate::client::client::PendingOutgoingMessage;

/// Events that can be broadcast from the client
#[derive(Debug, Clone)]
pub enum GameEvent {
    CharacterListReceived {
        account: String,
        characters: Vec<CharacterInfo>,
        num_slots: u32,
    },
    DDDInterrogation {
        language: u32,
        region: u32,
    },
}

#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub name: String,
    pub id: u32,
    pub delete_pending: bool,
}

/// Actions that event handlers can request the client to perform
#[derive(Debug)]
pub enum ClientAction {
    SendMessage(PendingOutgoingMessage),
    Disconnect,
}
