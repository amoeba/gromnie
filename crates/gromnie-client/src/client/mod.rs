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
pub mod event_bus;
pub mod refactored_event_bus;
#[allow(clippy::module_inception)]
mod client;
mod connection;
mod constants;
pub mod events;
mod messages;
mod protocol;
mod session;
