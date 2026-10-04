//! A headless facade over [`Client`].
//!
//! The types in [`crate::client`] are deliberately low-level: you construct a
//! `Client`, pump it yourself, and poll its state. That is what the TUI, the
//! runner, and the iOS bridge need, and it is not what a script or a bot
//! wants. This module wraps the driver loop and the event stream so a caller
//! can do:
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use gromnie_client::api::GromnieClient;
//!
//! let client = GromnieClient::builder()
//!     .with_server("play.example.com:9000")
//!     .with_account("account", "password")
//!     .connect()
//!     .await?;
//!
//! for character in client.list_characters() {
//!     println!("{}", character.name);
//! }
//!
//! client.enter_world("Character Name").await?;
//! client.say("hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Two conventions worth knowing:
//!
//! * **Mutations never take the client lock.** The driver loop holds the write
//!   lock across `recv_packet` and across a one-second sleep in the handshake,
//!   so any method that locked would block for seconds. Commands go through
//!   unbounded channels instead.
//! * **`connect()` returns at character select**, not at socket-up. A rejected
//!   login is a `connect()` error rather than an empty character list.
//!
//! This module is native-only. The wasm build drives its own loop over a WISP
//! transport and does not use this.

use std::sync::Arc;
use std::time::Duration;

use asheron_rs::gameactions::{CommunicationTalk, CommunicationTalkDirectByName};
use asheron_rs::message::GameActionMessage;
use asheron_rs::types::CharacterIdentity;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use tokio::task::JoinHandle;

use crate::client::{
    Client, ClientCommand, ClientError, ClientEvent, ClientHandle, ClientSystemEvent, ErrorScene,
    InWorldScene, LoopExit, Scene, spawn_client_loop,
};
use crate::transport::ClientTransport;

/// Capacity of the internal `ClientEvent` channel.
///
/// `Client::emit_protocol` uses `try_send`, which **silently drops** events when
/// the channel is full. The runner always has `EventWrapper` draining it. This
/// facade owns the receiver and republishes to a broadcast, so nothing is lost.
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Capacity of the broadcast handed to [`GromnieClient::subscribe`] callers.
const BROADCAST_CAPACITY: usize = 1024;

/// Default ceiling on how long `connect()` waits for character select.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default ceiling on how long `enter_world()` waits for the world transition.
const DEFAULT_ENTER_WORLD_TIMEOUT: Duration = Duration::from_secs(30);

/// Everything that can go wrong in the headless API.
///
/// Wraps [`ClientError`] rather than extending it: `ClientError` describes
/// protocol-level failures, while this also covers builder validation and
/// driver-loop lifecycle.
#[derive(Debug, Clone)]
pub enum ApiError {
    /// No server was configured. See [`GromnieClientBuilder::with_server`].
    MissingServer,
    /// No account name was configured.
    MissingAccount,
    /// No password was configured.
    MissingPassword,
    /// The server rejected the credentials.
    Authentication(String),
    /// The connection dropped and reconnection is disabled.
    ConnectionLost,
    /// The client entered an error scene.
    Client(ClientError),
    /// `connect()` did not reach character select in time.
    ConnectTimeout,
    /// `enter_world()` did not reach the world in time.
    EnterWorldTimeout,
    /// No character matched the given name.
    NoSuchCharacter(String),
    /// The driver loop stopped before the operation completed.
    LoopStopped(LoopExit),
    /// The driver task panicked or was cancelled.
    DriverFailed(String),
    /// A transport or channel error.
    Io(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::MissingServer => write!(f, "no server configured; call with_server()"),
            ApiError::MissingAccount => write!(f, "no account configured; call with_account()"),
            ApiError::MissingPassword => write!(f, "no password configured; call with_account()"),
            ApiError::Authentication(reason) => write!(f, "authentication rejected: {reason}"),
            ApiError::ConnectionLost => write!(f, "connection lost"),
            ApiError::Client(e) => write!(f, "{e}"),
            ApiError::ConnectTimeout => write!(f, "timed out waiting for character select"),
            ApiError::EnterWorldTimeout => {
                write!(f, "timed out waiting to enter the world")
            }
            ApiError::NoSuchCharacter(name) => write!(f, "no character named {name:?}"),
            ApiError::LoopStopped(reason) => write!(f, "client loop stopped: {reason:?}"),
            ApiError::DriverFailed(msg) => write!(f, "client driver failed: {msg}"),
            ApiError::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<ClientError> for ApiError {
    fn from(e: ClientError) -> Self {
        ApiError::Client(e)
    }
}

/// Builder for a [`GromnieClient`].
///
/// ```no_run
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// use gromnie_client::api::GromnieClient;
///
/// let client = GromnieClient::builder()
///     .with_server("localhost:9000")
///     .with_account("test", "test")
///     .connect()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct GromnieClientBuilder {
    server: Option<String>,
    account: Option<String>,
    password: Option<String>,
    character: Option<String>,
    client_id: u32,
    reconnect: bool,
    login_timeout: Option<Duration>,
    connect_timeout: Duration,
    enter_world_timeout: Duration,
    transport: Option<Box<dyn ClientTransport>>,
}

impl Default for GromnieClientBuilder {
    fn default() -> Self {
        Self {
            server: None,
            account: None,
            password: None,
            character: None,
            client_id: 1,
            reconnect: false,
            login_timeout: None,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            enter_world_timeout: DEFAULT_ENTER_WORLD_TIMEOUT,
            transport: None,
        }
    }
}

impl GromnieClientBuilder {
    /// Start a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the login server as `"host"` or `"host:port"` (default port 9000).
    pub fn with_server(mut self, server: impl Into<String>) -> Self {
        self.server = Some(server.into());
        self
    }

    /// Set the login server from a separate host and port.
    pub fn with_server_addr(mut self, host: impl Into<String>, port: u16) -> Self {
        self.server = Some(format!("{}:{}", host.into(), port));
        self
    }

    /// Set the account credentials.
    pub fn with_account(mut self, account: impl Into<String>, password: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self.password = Some(password.into());
        self
    }

    /// Auto-login as this character as soon as the character list arrives.
    ///
    /// Equivalent to [`GromnieClient::enter_world`] but decided up front, and
    /// it cannot be used to switch characters after connecting.
    pub fn with_character(mut self, character: impl Into<String>) -> Self {
        self.character = Some(character.into());
        self
    }

    /// Set the client id reported in events. Defaults to 1.
    pub fn with_client_id(mut self, id: u32) -> Self {
        self.client_id = id;
        self
    }

    /// Enable automatic reconnection. Off by default.
    ///
    /// Note that reconnection resets the scene to `Connecting`, which discards
    /// character-select state; see [`crate::client::Scene`].
    pub fn with_reconnect(mut self, reconnect: bool) -> Self {
        self.reconnect = reconnect;
        self
    }

    /// Override how long the client waits for a handshake response before
    /// failing. Defaults to the client's own 20 seconds.
    pub fn with_login_timeout(mut self, timeout: Duration) -> Self {
        self.login_timeout = Some(timeout);
        self
    }

    /// Override how long [`GromnieClient::connect`] waits for character
    /// select. Defaults to 30 seconds.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Override how long [`GromnieClient::enter_world`] waits for the world
    /// transition. Defaults to 30 seconds.
    pub fn with_enter_world_timeout(mut self, timeout: Duration) -> Self {
        self.enter_world_timeout = timeout;
        self
    }

    /// Supply a custom transport instead of the default UDP socket.
    pub fn with_transport(mut self, transport: Box<dyn ClientTransport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Connect and wait until the character list is available.
    ///
    /// Returns `Ok` once the client reaches `Scene::CharacterSelect`. A
    /// rejected login, a dropped connection, a handshake timeout, or a driver
    /// failure all return `Err`.
    ///
    /// Protocol progress is driven by incoming handshake and login messages;
    /// there are no UI pacing sleeps in this path.
    pub async fn connect(self) -> Result<GromnieClient, ApiError> {
        let server = self.server.ok_or(ApiError::MissingServer)?;
        let account = self.account.clone().ok_or(ApiError::MissingAccount)?;
        let password = self.password.ok_or(ApiError::MissingPassword)?;

        let (raw_event_tx, raw_event_rx) = mpsc::channel::<ClientEvent>(EVENT_CHANNEL_CAPACITY);

        let (mut client, _legacy_actions) = match self.transport {
            Some(transport) => {
                Client::new_with_transport(
                    self.client_id,
                    server,
                    account.clone(),
                    password,
                    self.character,
                    raw_event_tx,
                    self.reconnect,
                    transport,
                )
                .await
            }
            None => {
                Client::new_with_reconnect(
                    self.client_id,
                    server,
                    account.clone(),
                    password,
                    self.character,
                    raw_event_tx,
                    self.reconnect,
                )
                .await
            }
        };

        if let Some(timeout) = self.login_timeout {
            client.set_login_timeout(timeout);
        }

        // There is no progress UI to pace before the initial login request.
        let handle = spawn_client_loop(Arc::new(RwLock::new(client)), Duration::ZERO).await;

        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (terminal_tx, terminal_rx) = watch::channel(None);
        let pump = tokio::spawn(pump_events(
            raw_event_rx,
            broadcast_tx.clone(),
            terminal_tx.clone(),
        ));

        let mut client = GromnieClient {
            handle,
            account,
            characters: Vec::new(),
            enter_world_timeout: self.enter_world_timeout,
            broadcast_tx,
            _terminal_tx: terminal_tx,
            terminal_rx,
            pump: Some(pump),
        };

        client.await_character_select(self.connect_timeout).await?;
        Ok(client)
    }
}

/// A connected headless client.
///
/// Obtained from [`GromnieClientBuilder::connect`]. Dropping it shuts the driver
/// loop down.
pub struct GromnieClient {
    handle: ClientHandle,
    account: String,
    /// Snapshot taken at character select. `Scene` stops carrying the list once
    /// the client enters the world, so we keep our own copy.
    characters: Vec<CharacterIdentity>,
    enter_world_timeout: Duration,
    broadcast_tx: broadcast::Sender<ClientEvent>,
    /// Held so the terminal-error channel never closes while the client lives,
    /// which keeps `wait_for` from spinning on a disconnected sender. The
    /// leading underscore marks it as deliberately write-only.
    _terminal_tx: watch::Sender<Option<ApiError>>,
    terminal_rx: watch::Receiver<Option<ApiError>>,
    pump: Option<JoinHandle<()>>,
}

impl GromnieClient {
    /// Start building a client.
    pub fn builder() -> GromnieClientBuilder {
        GromnieClientBuilder::new()
    }

    /// The characters on this account.
    ///
    /// Synchronous because the list is already known by the time `connect()`
    /// returns. Characters pending deletion are excluded, matching the
    /// auto-login rules in the client.
    pub fn list_characters(&self) -> Vec<CharacterIdentity> {
        active_characters(&self.characters).cloned().collect()
    }

    /// Look up a character by name, case-insensitively.
    pub fn character(&self, name: &str) -> Option<&CharacterIdentity> {
        find_character(&self.characters, name)
    }

    /// The current scene snapshot. Cheap; does not touch the client lock.
    pub fn scene(&self) -> Scene {
        self.handle.scene()
    }

    /// Subscribe to protocol, state, and system events.
    ///
    /// Events are dropped for subscribers that fall behind; internal waits use
    /// the scene and terminal-error channels instead, so a lagging subscriber
    /// cannot wedge this client.
    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Send a general chat message, equivalent to `/say`.
    pub async fn say(&self, message: impl Into<String>) -> Result<(), ApiError> {
        self.handle
            .send_game_action(GameActionMessage::CommunicationTalk(CommunicationTalk {
                message: message.into(),
            }))
            .map_err(|_| ApiError::ConnectionLost)
    }

    /// Send a private message to another player by name.
    pub async fn tell(
        &self,
        recipient: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<(), ApiError> {
        self.handle
            .send_game_action(GameActionMessage::CommunicationTalkDirectByName(
                CommunicationTalkDirectByName {
                    message: message.into(),
                    target_name: recipient.into(),
                },
            ))
            .map_err(|_| ApiError::ConnectionLost)
    }

    /// Enter the world as `name` and resolve once the client is in-world.
    ///
    /// The name match is case-insensitive. Resolves with the resulting
    /// [`InWorldScene`].
    pub async fn enter_world(&self, name: &str) -> Result<InWorldScene, ApiError> {
        let character = self
            .character(name)
            .cloned()
            .ok_or_else(|| ApiError::NoSuchCharacter(name.to_string()))?;

        self.handle
            .send(ClientCommand::EnterWorld {
                character_id: character.character_id.0,
                character_name: character.name.clone(),
                account: self.account.clone(),
            })
            .map_err(|_| {
                ApiError::LoopStopped(
                    self.handle
                        .exit_reason()
                        .unwrap_or(LoopExit::DisconnectRequested),
                )
            })?;

        let target_id = character.character_id.0;
        self.wait_for_scene(self.enter_world_timeout, move |scene| match scene {
            Scene::InWorld(world) if world.character_id == target_id => Some(world.clone()),
            _ => None,
        })
        .await
    }

    /// Stop the driver loop and wait for it to finish.
    ///
    /// The client is unusable afterwards; this takes `&self` so it can be
    /// called from a shared reference, and the client's `Drop` performs the same
    /// shutdown if you skip this.
    pub async fn disconnect(&self) -> Result<(), ApiError> {
        self.handle.shutdown();
        match self.handle.wait_for_exit().await {
            Some(LoopExit::Shutdown) | Some(LoopExit::DisconnectRequested) | None => Ok(()),
            Some(reason) => Err(ApiError::LoopStopped(reason)),
        }
    }

    /// Wait for character select, then snapshot the character list.
    async fn await_character_select(&mut self, timeout: Duration) -> Result<(), ApiError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut scene = self.handle.subscribe_scene();
        let mut terminal = self.terminal_rx.clone();

        loop {
            match scene.borrow_and_update().clone() {
                Scene::CharacterSelect(select) => {
                    self.characters = select.characters;
                    return Ok(());
                }
                Scene::Error(scene) => return Err(error_from_scene(scene)),
                _ => {}
            }

            if let Some(error) = terminal.borrow_and_update().clone() {
                return Err(error);
            }
            if let Some(reason) = self.handle.exit_reason() {
                return Err(ApiError::LoopStopped(reason));
            }

            tokio::select! {
                changed = scene.changed() => {
                    changed.map_err(|_| ApiError::DriverFailed("scene channel closed".into()))?;
                }
                _ = terminal.wait_for(|slot| slot.is_some()) => {}
                _ = tokio::time::sleep_until(deadline) => return Err(ApiError::ConnectTimeout),
            }
        }
    }

    /// Wait until `predicate` produces a value, or the timeout elapses.
    async fn wait_for_scene<T>(
        &self,
        timeout: Duration,
        predicate: impl Fn(&Scene) -> Option<T>,
    ) -> Result<T, ApiError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut scene = self.handle.subscribe_scene();
        let mut terminal = self.terminal_rx.clone();

        loop {
            if let Some(value) = predicate(&scene.borrow_and_update()) {
                return Ok(value);
            }
            // An error scene aborts the wait with the real cause rather than
            // the generic `LoopStopped` that the exit reason would give.
            if let Scene::Error(error) = scene.borrow_and_update().clone() {
                return Err(error_from_scene(error));
            }
            if let Some(error) = terminal.borrow_and_update().clone() {
                return Err(error);
            }
            if let Some(reason) = self.handle.exit_reason() {
                return Err(ApiError::LoopStopped(reason));
            }

            tokio::select! {
                changed = scene.changed() => {
                    changed.map_err(|_| ApiError::DriverFailed("scene channel closed".into()))?;
                }
                _ = terminal.wait_for(|slot| slot.is_some()) => {}
                _ = tokio::time::sleep_until(deadline) => return Err(ApiError::EnterWorldTimeout),
            }
        }
    }
}

impl std::fmt::Debug for GromnieClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately omits the channels and the client itself; both are
        // opaque, and the scene is the only useful summary.
        f.debug_struct("GromnieClient")
            .field("scene", &self.handle.scene())
            .field("characters", &self.characters.len())
            .field("running", &self.handle.exit_reason().is_none())
            .finish()
    }
}

impl Drop for GromnieClient {
    fn drop(&mut self) {
        self.handle.shutdown();
        if let Some(pump) = self.pump.take() {
            pump.abort();
        }
    }
}

/// The characters that are eligible to be logged into.
///
/// A character with a non-zero `seconds_greyed_out` is pending deletion and
/// cannot be entered. This mirrors the client's own auto-login filter.
fn active_characters(characters: &[CharacterIdentity]) -> impl Iterator<Item = &CharacterIdentity> {
    characters.iter().filter(|c| c.seconds_greyed_out == 0)
}

/// Find an enterable character by name, ignoring ASCII case.
fn find_character<'a>(
    characters: &'a [CharacterIdentity],
    name: &str,
) -> Option<&'a CharacterIdentity> {
    active_characters(characters).find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Drain the client's event channel, republish it, and record terminal errors.
///
/// Owning this receiver is what keeps `Client::emit_protocol`'s `try_send` from
/// dropping events: with no reader attached the channel fills and events are
/// discarded.
///
/// Only `Disconnected` is turned into a terminal error here. Login rejections
/// are **not**: `ClientSystemEvent::AuthenticationFailed` is overloaded — the
/// client emits it both for a real rejection (`message_handlers.rs:153`) and for
/// a connect timeout (`client.rs:757`). Trusting it made an unreachable server
/// report as `Authentication("Connection timeout")`. Both paths set the scene
/// to `Scene::Error`, so [`error_from_scene`] is authoritative and unambiguous.
async fn pump_events(
    mut rx: mpsc::Receiver<ClientEvent>,
    broadcast_tx: broadcast::Sender<ClientEvent>,
    terminal_tx: watch::Sender<Option<ApiError>>,
) {
    while let Some(event) = rx.recv().await {
        if let ClientEvent::System(ClientSystemEvent::Disconnected {
            will_reconnect: false,
            ..
        }) = &event
        {
            record_terminal(&terminal_tx, ApiError::ConnectionLost);
        }

        // No subscribers yet is normal and not an error.
        let _ = broadcast_tx.send(event);
    }
}

/// Convert an error scene into the most specific `ApiError` available.
///
/// A rejected login is by far the most common failure, so it gets its own
/// variant instead of being buried in [`ApiError::Client`].
fn error_from_scene(scene: ErrorScene) -> ApiError {
    match scene.error {
        ClientError::Authentication(reason) => ApiError::Authentication(reason),
        other => ApiError::Client(other),
    }
}

/// Record the first terminal failure; later ones do not overwrite it.
fn record_terminal(slot: &watch::Sender<Option<ApiError>>, error: ApiError) {
    slot.send_if_modified(|existing| {
        if existing.is_none() {
            *existing = Some(error);
            true
        } else {
            false
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use asheron_rs::types::ObjectId;

    fn character(id: u32, name: &str, greyed_out: u32) -> CharacterIdentity {
        CharacterIdentity {
            character_id: ObjectId(id),
            name: name.to_string(),
            seconds_greyed_out: greyed_out,
        }
    }

    /// The facade must be usable from a spawned task, which is the whole point
    /// of not exposing the client lock.
    #[test]
    fn client_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GromnieClient>();
        assert_send::<GromnieClientBuilder>();
    }

    #[test]
    fn builder_reports_missing_fields() {
        let error =
            block_on(GromnieClient::builder().connect()).expect_err("missing server should fail");
        assert!(matches!(error, ApiError::MissingServer));

        let error = block_on(
            GromnieClient::builder()
                .with_server("localhost:9000")
                .connect(),
        )
        .expect_err("missing account should fail");
        assert!(matches!(error, ApiError::MissingAccount));
    }

    #[test]
    fn with_server_addr_formats_host_port() {
        let builder = GromnieClientBuilder::new().with_server_addr("example.com", 9001);
        assert_eq!(builder.server.as_deref(), Some("example.com:9001"));
    }

    #[test]
    fn greyed_out_characters_are_not_listed() {
        let characters = vec![
            character(1, "Alive", 0),
            character(2, "Pending", 42),
            character(3, "AlsoAlive", 0),
        ];

        let names: Vec<_> = active_characters(&characters)
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["Alive", "AlsoAlive"]);
    }

    #[test]
    fn character_lookup_ignores_case() {
        let characters = vec![character(7, "Ari", 0), character(8, "Bo", 0)];

        assert_eq!(
            find_character(&characters, "ari").unwrap().character_id.0,
            7
        );
        assert_eq!(
            find_character(&characters, "ARI").unwrap().character_id.0,
            7
        );
        assert_eq!(find_character(&characters, "Bo").unwrap().character_id.0, 8);
        assert!(find_character(&characters, "Cy").is_none());
    }

    #[test]
    fn character_lookup_skips_pending_deletion() {
        let characters = vec![character(7, "Ari", 60)];
        assert!(find_character(&characters, "Ari").is_none());
    }

    #[test]
    fn first_terminal_error_wins() {
        let (tx, mut rx) = watch::channel(None);

        record_terminal(&tx, ApiError::ConnectionLost);
        assert!(matches!(
            *rx.borrow_and_update(),
            Some(ApiError::ConnectionLost)
        ));

        // A later, more specific failure must not mask the original cause.
        record_terminal(&tx, ApiError::Authentication("bad password".into()));
        assert!(matches!(
            *rx.borrow_and_update(),
            Some(ApiError::ConnectionLost)
        ));
    }

    #[test]
    fn terminal_channel_resolves_waiters() {
        let (tx, rx) = watch::channel(None);

        let observed = block_on(async move {
            let mut rx = rx;
            let waiter = tokio::spawn(async move {
                rx.wait_for(|slot| slot.is_some()).await.unwrap();
                rx.borrow().clone()
            });

            record_terminal(&tx, ApiError::ConnectTimeout);
            waiter.await.unwrap()
        });

        assert!(matches!(observed, Some(ApiError::ConnectTimeout)));
    }

    #[test]
    fn terminal_channel_resolves_for_waiters_that_arrive_late() {
        let (tx, rx) = watch::channel(None);
        record_terminal(&tx, ApiError::Authentication("bad password".into()));

        // A waiter that starts after the fact must not block forever. This is
        // why the facade holds its own `watch::Sender` instead of relying on
        // the pump task staying alive.
        let observed = block_on(async move {
            let mut rx = rx;
            rx.wait_for(|slot| slot.is_some()).await.unwrap();
            rx.borrow().clone()
        });
        assert!(matches!(observed, Some(ApiError::Authentication(_))));
    }

    #[test]
    fn error_scenes_map_to_the_most_specific_error() {
        // A rejected login is common enough to deserve its own variant...
        let rejected = error_from_scene(ErrorScene::new(
            ClientError::Authentication("wrong password".into()),
            false,
        ));
        assert!(matches!(rejected, ApiError::Authentication(ref r) if r == "wrong password"));

        // ...but a timeout must NOT be reported as an auth failure. Live
        // testing showed `ClientSystemEvent::AuthenticationFailed` carrying
        // "Connection timeout - server not responding", which this mapping
        // deliberately ignores in favour of the scene.
        let timed_out = error_from_scene(ErrorScene::new(ClientError::LoginTimeout, false));
        assert!(matches!(
            timed_out,
            ApiError::Client(ClientError::LoginTimeout)
        ));

        let patching = error_from_scene(ErrorScene::new(ClientError::PatchingTimeout, false));
        assert!(matches!(
            patching,
            ApiError::Client(ClientError::PatchingTimeout)
        ));
    }

    #[test]
    fn api_error_messages_are_descriptive() {
        assert!(ApiError::MissingServer.to_string().contains("with_server"));
        assert!(
            ApiError::Authentication("nope".into())
                .to_string()
                .contains("nope")
        );
        assert!(
            ApiError::Client(ClientError::LoginTimeout)
                .to_string()
                .contains("login timed out")
        );
        assert!(
            ApiError::NoSuchCharacter("Ghost".into())
                .to_string()
                .contains("Ghost")
        );
    }

    /// Minimal blocking bridge so the builder-validation test stays a plain
    /// `#[test]` instead of needing `#[tokio::test]`.
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(future)
    }
}
