use crate::client::client::PendingOutgoingMessage;

/// Events that can be broadcast from the client
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
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
    /// Character login succeeded - received LoginComplete notification
    LoginSucceeded {
        character_id: u32,
        character_name: String,
    },
    /// Character login failed
    LoginFailed {
        reason: String,
    },
    /// Object created in the game world
    CreateObject {
        object_id: u32,
        object_name: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Log in as a specific character
    LoginCharacter { character_id: u32, character_name: String, account: String },
    /// Send LoginComplete notification to server after receiving initial objects
    SendLoginComplete,
}
