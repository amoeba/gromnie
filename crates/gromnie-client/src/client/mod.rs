// Re-export main types
pub use self::client::Client;
pub use self::command::{ClientCommand, ClientLoopStopped, ClientSender, LoopExit};
pub use self::connection::ServerInfo;
pub use self::consumer::{ConsumerContext, ConsumerFactory};
#[cfg(not(target_arch = "wasm32"))]
pub use self::driver::{ClientHandle, spawn_client_loop};
pub use self::messages::{OutgoingMessage, OutgoingMessageContent};
pub use self::protocol::{C2SPacketExt, CustomLoginRequest};
pub use self::scene::{
    CharacterCreateScene, CharacterSelectScene, ClientError, ConnectingProgress, ConnectingScene,
    EnteringWorldState, ErrorScene, InWorldScene, PatchingProgress, Scene,
};
pub use self::session::{Account, ClientSession, ConnectionState, SessionState};

// Re-export event types from gromnie-events for compatibility
pub use gromnie_events::{ClientEvent, ClientStateEvent, ClientSystemEvent, ProtocolEvent};

pub mod ace_protocol;
#[allow(clippy::module_inception)]
mod client;
mod command;
mod connection;
mod constants;
mod consumer;
// The driver loop uses `tokio::time` deadlines, which have no timer driver on
// wasm32 (see `crate::instant` for the same reason). `gromnie-web` drives its
// own loop over a WISP transport, draining the same `ClientCommand` channel.
#[cfg(not(target_arch = "wasm32"))]
pub mod driver;
pub mod game_event_handler;
pub mod message_handler;
mod message_handlers;
mod messages;
mod protocol;
mod protocol_conversions;
mod scene;
mod session;
