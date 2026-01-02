// Re-export main types
pub use self::client::{
    Client, ClientFailureReason, ClientState, ConnectingProgress, PatchingProgress,
};
pub use self::connection::ServerInfo;
pub use self::constants::UI_DELAY_MS;
pub use self::messages::{OutgoingMessage, OutgoingMessageContent};
pub use self::protocol::{C2SPacketExt, CustomLoginRequest};
pub use self::session::{Account, SessionState};

pub mod ace_protocol;
#[allow(clippy::module_inception)]
mod client;
mod connection;
mod constants;
pub mod events;
pub mod game_event_handler;
mod game_event_handlers;
pub mod message_handler;
mod message_handlers;
mod messages;
mod protocol;
mod session;
