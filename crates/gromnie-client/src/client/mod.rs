// Re-export main types
pub use self::client::Client;
pub use self::connection::ServerInfo;
pub use self::constants::UI_DELAY_MS;
pub use self::messages::{OutgoingMessage, OutgoingMessageContent};
pub use self::protocol::{C2SPacketExt, CustomLoginRequest};
pub use self::scene::{
    CharacterCreateScene, CharacterSelectScene, ClientError, ConnectingProgress, ConnectingScene,
    EnteringWorldState, ErrorScene, InWorldScene, PatchingProgress, Scene,
};
pub use self::session::{Account, ClientSession, ConnectionState, SessionState};

// Re-export event types from gromnie-events for compatibility
pub use gromnie_events::{
    ClientEvent, ClientStateEvent, ClientSystemEvent, SimpleClientAction,
    SimpleGameEvent as GameEvent,
};
// Re-export internal types
pub use types::ClientAction;

pub mod ace_protocol;
#[allow(clippy::module_inception)]
mod client;
mod connection;
mod constants;
pub mod game_event_handler;
pub mod message_handler;
mod message_handlers;
mod messages;
mod protocol;
mod protocol_conversions;
mod scene;
mod session;
pub mod types;
