use serde::{Deserialize, Serialize};

/// Character information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub id: u32,
    pub delete_pending: bool,
}

/// Simplified versions of acprotocol GameEvent/OrderedGameEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleGameEvent {
    CharacterListReceived {
        account: String,
        characters: Vec<CharacterInfo>,
        num_slots: u32,
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
    /// Character error received from server
    CharacterError {
        error_code: u32,
        error_message: String,
    },
    /// Object created in the game world
    CreateObject {
        object_id: u32,
        object_name: String,
    },
    /// Chat message received from server
    ChatMessageReceived {
        message: String,
        message_type: u32,
    },
    CreatePlayer {
        character_id: u32,
    },
    /// Progress update for connecting phase
    ConnectingSetProgress {
        progress: f64,
    },
    /// Progress update for patching/updating phase
    UpdatingSetProgress {
        progress: f64,
    },
}
