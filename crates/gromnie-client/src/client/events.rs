use crate::client::messages::OutgoingMessageContent;

/// Direction of network message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MessageDirection {
    Sent,
    Received,
}

/// Events that can be broadcast from the client
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Chat message received from server
    ChatMessageReceived {
        message: String,
        message_type: u32,
    },
    /// Network message sent or received (for debug view)
    NetworkMessage {
        direction: MessageDirection,
        message_type: String,
    },
    /// Set connecting progress (for authentication)
    ConnectingSetProgress {
        progress: f64,
    },
    /// Set updating progress (for DDD messages)
    UpdatingSetProgress {
        progress: f64,
    },
    /// Signal connecting phase has started
    ConnectingStart,
    /// Signal connecting phase is done
    ConnectingDone,
    /// Authentication succeeded - received ConnectRequest from server
    AuthenticationSucceeded,
    /// Authentication failed - login credentials rejected
    AuthenticationFailed {
        reason: String,
    },
    /// Signal updating phase has started
    UpdatingStart,
    /// Signal updating phase is done
    UpdatingDone,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub id: u32,
    pub delete_pending: bool,
}

/// Client state transition events
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClientStateEvent {
    StateTransition { from: String, to: String, client_id: u32 },
    ClientFailed { reason: String, client_id: u32 },
}

/// System events that originate from the client (before enrichment by runner)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClientSystemEvent {
    AuthenticationSucceeded,
    AuthenticationFailed { reason: String },
    ConnectingStarted,
    ConnectingDone,
    UpdatingStarted,
    UpdatingDone,
    LoginSucceeded { character_id: u32, character_name: String },
}

/// Events emitted by the client without context/metadata
/// These will be wrapped and enriched by the runner's EventWrapper
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Game(GameEvent),
    State(ClientStateEvent),
    System(ClientSystemEvent),
}

/// Actions that event handlers can request the client to perform
#[derive(Debug)]
pub enum ClientAction {
    SendMessage(Box<OutgoingMessageContent>),
    Disconnect,
    /// Log in as a specific character
    LoginCharacter {
        character_id: u32,
        character_name: String,
        account: String,
    },
    /// Send LoginComplete notification to server after receiving initial objects
    SendLoginComplete,
    /// Send a chat message
    SendChatMessage {
        message: String,
    },
    /// Reload scripts (internal use for hot-reload)
    ReloadScripts {
        script_dir: std::path::PathBuf,
    },
    /// Log a message from a script
    LogScriptMessage {
        script_id: String,
        message: String,
    },
}
