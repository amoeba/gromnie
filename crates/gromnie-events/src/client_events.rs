use crate::protocol_events::ProtocolEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Protocol(ProtocolEvent),
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
    /// Progress within the connecting phase, from 0.0 to 1.0
    ConnectingProgress {
        progress: f64,
    },
    UpdatingStarted,
    UpdatingDone,
    /// Progress within the updating/patching phase, from 0.0 to 1.0
    UpdatingProgress {
        progress: f64,
    },
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
