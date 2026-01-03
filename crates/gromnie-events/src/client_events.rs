use crate::simple_game_events::SimpleGameEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Game(SimpleGameEvent),
    State(ClientStateEvent),
    System(ClientSystemEvent),
}

/// System events that originate from the client (lifecycle events)
#[derive(Debug, Clone)]
pub enum ClientSystemEvent {
    AuthenticationSucceeded,
    AuthenticationFailed {
        reason: String,
    },
    ConnectingStarted,
    ConnectingDone,
    UpdatingStarted,
    UpdatingDone,
    LoginSucceeded {
        character_id: u32,
        character_name: String,
    },
    /// Connection was lost
    Disconnected {
        will_reconnect: bool,
        reconnect_attempt: u32,
        delay_secs: u64,
    },
    /// Attempting to reconnect
    Reconnecting {
        attempt: u32,
        delay_secs: u64,
    },
}

/// State of the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientStateEvent {
    Connecting,
    Connected,
    ConnectingFailed { reason: String },
    Patching,
    Patched,
    PatchingFailed { reason: String },
    CharacterSelect,
    EnteringWorld,
    InWorld,
    ExitingWorld,
    CharacterError,
}
