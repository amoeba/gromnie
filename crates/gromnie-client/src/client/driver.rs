//! The client driver loop.
//!
//! This is the single place where a [`Client`] is pumped: receive a packet,
//! process it, drain queued work, flush outgoing messages, and service
//! keepalives and timeouts. It was previously private to
//! `gromnie-runner`; it lives here so that any embedder — the runner, the
//! TUI, a headless facade — drives the client identically.
//!
//! Consumers interact through [`ClientHandle`], which exposes channels and a
//! [`Scene`] watch. It deliberately exposes no `Arc<RwLock<Client>>`: callers
//! that need direct access (the scripting host) keep their own clone of the
//! arc they already hold, and new callers are pushed toward channels, which
//! avoids contending on a lock that this loop holds across `recv_packet`.
//!
//! Note the module name: this is not `loop.rs`, since `loop` is a keyword.

use std::sync::Arc;

use asheron_rs::message::GameActionMessage;
use tokio::sync::{RwLock, mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::client::Client;
use crate::client::scene::{ClientError, ErrorScene, Scene};

/// Session-level commands issued by a [`ClientHandle`].
///
/// Intentionally narrow. Game-level operations (`say`, `tell`, movement) are
/// sent as [`GameActionMessage`]s over `game_action_tx`; UI-level observation
/// happens on the event channel. This channel carries only the transitions
/// that are not game actions.
#[derive(Debug, Clone)]
pub enum ClientCommand {
    /// Request entry into the world for a character.
    EnterWorld {
        character_id: u32,
        character_name: String,
        account: String,
    },
    /// Send the `LoginComplete` notification once initial world state arrives.
    SendLoginComplete,
    /// Disconnect and stop the driver loop.
    Disconnect,
}

/// Why the driver loop stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopExit {
    /// Shutdown was requested through the shutdown channel.
    Shutdown,
    /// The client entered an unrecoverable error state (see `ClientError`).
    Failed,
    /// Reconnection is disabled or has exhausted its attempts.
    ReconnectUnavailable,
    /// The initial `LoginRequest` could not be sent.
    LoginRequestFailed,
    /// A [`ClientCommand::Disconnect`] was received.
    DisconnectRequested,
    /// The driver task unwound (panicked) or was cancelled. Carries the detail.
    ///
    /// Without this the loop would stop silently and anything waiting on
    /// [`ClientHandle::wait_for_exit`] would hang forever.
    TaskFailed(String),
}

/// The driver loop is gone, so the command was never queued.
///
/// Returned instead of handing the command back: there is nothing to retry
/// against, and it keeps the large `GameActionMessage` out of the error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientLoopStopped;

impl std::fmt::Display for ClientLoopStopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "client driver loop is no longer running")
    }
}

impl std::error::Error for ClientLoopStopped {}

/// Handle to a running client driver loop.
///
/// Obtained from [`spawn_client_loop`]. Dropping the handle does not stop the
/// loop; call [`ClientHandle::shutdown`] or send [`ClientCommand::Disconnect`].
pub struct ClientHandle {
    commands: mpsc::UnboundedSender<ClientCommand>,
    game_actions: mpsc::UnboundedSender<GameActionMessage>,
    scene: watch::Receiver<Scene>,
    exit: watch::Receiver<Option<LoopExit>>,
    shutdown: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl ClientHandle {
    /// Send a session-level command. Never blocks and never takes the client
    /// lock, so it is safe to call from any task.
    pub fn send(&self, command: ClientCommand) -> Result<(), ClientLoopStopped> {
        self.commands.send(command).map_err(|_| ClientLoopStopped)
    }

    /// Send a game action directly, bypassing `SimpleClientAction`.
    ///
    /// Prefer this over [`ClientHandle::send`] for anything expressible as a
    /// `GameActionMessage` (chat, movement, trade, magic).
    pub fn send_game_action(&self, action: GameActionMessage) -> Result<(), ClientLoopStopped> {
        self.game_actions
            .send(action)
            .map_err(|_| ClientLoopStopped)
    }

    /// Subscribe to scene snapshots. Updated at the end of every loop
    /// iteration and immediately when reconnection resets the scene.
    pub fn subscribe_scene(&self) -> watch::Receiver<Scene> {
        self.scene.clone()
    }

    /// The most recent scene snapshot. Cheap, and never blocks on the lock
    /// because it reads the watch channel rather than the client.
    pub fn scene(&self) -> Scene {
        self.scene.borrow().clone()
    }

    /// Why the loop stopped, or `None` while it is still running.
    pub fn exit_reason(&self) -> Option<LoopExit> {
        self.exit.borrow().clone()
    }

    /// Resolve once the loop has stopped, with the reason.
    ///
    /// Backed by a watch channel, so it resolves immediately if the loop has
    /// already stopped and cannot miss a wakeup. Also resolves if the task
    /// panicked, via [`ExitGuard`].
    pub async fn wait_for_exit(&self) -> Option<LoopExit> {
        let mut rx = self.exit.clone();
        rx.wait_for(|slot| slot.is_some()).await.ok()?;
        rx.borrow().clone()
    }

    /// Request shutdown. The loop stops on its next iteration.
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }

    /// A clone of the shutdown sender, for bridging an external signal (a
    /// Ctrl+C watcher, a UI quit button) into this loop.
    pub fn shutdown_sender(&self) -> watch::Sender<bool> {
        self.shutdown.clone()
    }

    /// Request shutdown and wait for the loop to finish.
    ///
    /// Returns `None` only if the task ended without recording a reason.
    pub async fn join(mut self) -> Option<LoopExit> {
        self.shutdown();
        let outcome = (&mut self.join).await;
        let recorded = self.exit_reason();

        match outcome {
            Ok(()) => recorded,
            // Prefer the panic payload over `ExitGuard`'s generic message.
            Err(join_error) if join_error.is_panic() => {
                Some(LoopExit::TaskFailed(join_error.to_string()))
            }
            Err(_) => Some(
                recorded
                    .unwrap_or_else(|| LoopExit::TaskFailed("driver task was cancelled".into())),
            ),
        }
    }
}

/// Spawn the driver loop for `client`.
///
/// The loop owns the only shutdown channel; use [`ClientHandle::shutdown`] or
/// [`ClientHandle::shutdown_sender`] to stop it. It exits when that channel
/// fires, when a [`ClientCommand::Disconnect`] arrives, when the client enters
/// an unrecoverable error state, or when reconnection becomes impossible.
///
/// `initial_login_delay` exists because the runner waits a beat before its
/// first `LoginRequest` to make its progress UI visible (`UI_DELAY_MS`).
/// Headless callers should pass `Duration::ZERO`.
pub async fn spawn_client_loop(
    client: Arc<RwLock<Client>>,
    initial_login_delay: std::time::Duration,
) -> ClientHandle {
    // Read the initial snapshot and the game-action sender once, before the
    // loop starts contending for the lock.
    let (game_actions, initial_scene) = {
        let guard = client.read().await;
        (guard.game_action_tx.clone(), guard.scene.clone())
    };

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (scene_tx, scene_rx) = watch::channel(initial_scene);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (exit_tx, exit_rx) = watch::channel(None);

    let join = tokio::spawn(async move {
        run(
            client,
            command_rx,
            scene_tx,
            exit_tx,
            shutdown_rx,
            initial_login_delay,
        )
        .await;
    });

    ClientHandle {
        commands: command_tx,
        game_actions,
        scene: scene_rx,
        exit: exit_rx,
        shutdown: shutdown_tx,
        join,
    }
}

/// Record why the loop stopped. The first reason wins.
fn set_exit(exit: &watch::Sender<Option<LoopExit>>, reason: LoopExit) {
    exit.send_if_modified(|slot| {
        if slot.is_none() {
            *slot = Some(reason);
            true
        } else {
            false
        }
    });
}

/// Records a failure reason if [`run`] unwinds.
///
/// Every normal exit path calls [`set_exit`] first, and `set_exit` is
/// first-wins, so this only ever fires for a panic or a cancellation.
struct ExitGuard {
    exit: watch::Sender<Option<LoopExit>>,
}

impl Drop for ExitGuard {
    fn drop(&mut self) {
        set_exit(
            &self.exit,
            LoopExit::TaskFailed("driver task panicked or was cancelled".to_string()),
        );
    }
}

/// Apply a batch of queued commands. Returns an exit reason if the loop
/// should stop.
fn apply_commands(
    client: &mut Client,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
) -> Option<LoopExit> {
    while let Ok(command) = command_rx.try_recv() {
        match command {
            ClientCommand::EnterWorld {
                character_id,
                character_name,
                account,
            } => {
                if let Err(e) =
                    client.attempt_character_login(character_id, character_name, account)
                {
                    error!(target: "net", "ClientCommand::EnterWorld failed: {}", e);
                }
            }
            ClientCommand::SendLoginComplete => client.send_login_complete_notification(),
            ClientCommand::Disconnect => {
                info!(target: "net", "ClientCommand::Disconnect received");
                client.scene = Scene::Error(ErrorScene::new(
                    ClientError::ConnectionFailed("Disconnected by client action".to_string()),
                    true,
                ));
                return Some(LoopExit::DisconnectRequested);
            }
        }
    }
    None
}

async fn run(
    client: Arc<RwLock<Client>>,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    scene_tx: watch::Sender<Scene>,
    exit: watch::Sender<Option<LoopExit>>,
    mut shutdown_rx: watch::Receiver<bool>,
    initial_login_delay: std::time::Duration,
) {
    // Armed before anything can fail, so an early panic still unblocks anyone
    // waiting on the exit reason.
    let _exit_guard = ExitGuard { exit: exit.clone() };

    let client_id = client.read().await.client_id();
    info!(target: "net", "Client {} network loop started", client_id);

    if !initial_login_delay.is_zero() {
        // The runner delays the first LoginRequest so its progress UI is
        // visible. Headless callers pass Duration::ZERO.
        tokio::time::sleep(initial_login_delay).await;
    }

    if let Err(e) = client.write().await.do_login().await {
        error!(target: "net", "Failed to send initial LoginRequest: {}", e);
        set_exit(&exit, LoopExit::LoginRequestFailed);
        return;
    }
    info!(target: "net", "Initial LoginRequest sent - entering state machine loop");

    let mut buf = [0u8; 1024];
    let mut last_keepalive = tokio::time::Instant::now();
    // Send keepalive every 5 seconds to stay well within the server's timeout window
    // (Server timeout is configurable but defaults to 60s for gameplay, could be as low as 10s)
    let keepalive_interval = tokio::time::Duration::from_secs(5);

    // Tick interval for checking retries and timeouts
    let tick_interval = tokio::time::Duration::from_millis(100); // Check every 100ms
    let mut last_tick = tokio::time::Instant::now();

    // `break` carries the reason out so the final scene is published and the exit
    // reason recorded exactly once, in a deterministic order.
    let reason = loop {
        // Apply queued session commands before servicing the socket, so a
        // command issued between iterations takes effect on this one.
        {
            let mut guard = client.write().await;
            if let Some(reason) = apply_commands(&mut guard, &mut command_rx) {
                break reason;
            }
        }

        tokio::select! {
            // Add a timeout to transport recv so we can respond to shutdown signals
            recv_result = async {
                let mut client_guard = client.write().await;
                tokio::time::timeout(tokio::time::Duration::from_millis(100), client_guard.recv_packet(&mut buf)).await
            } => {
                match recv_result {
                    Ok(Ok((size, peer))) => {
                        let mut client_guard = client.write().await;
                        client_guard.process_packet(&buf[..size], size, &peer).await;

                        if client_guard.has_messages() {
                            client_guard.process_messages();
                        }

                        client_guard.process_actions();
                        client_guard.process_game_actions();

                        if client_guard.has_pending_outgoing_messages()
                            && let Err(e) = client_guard.send_pending_messages().await {
                            error!(target: "net", "Failed to send pending messages: {}", e);
                            }
                    }
                    Ok(Err(e)) => {
                        error!(target: "net", "Error in receive loop: {}", e);
                        // Always transition to disconnected state on transport error
                        client.write().await.enter_disconnected();
                    }
                    Err(_) => {
                        // Timeout - this is normal, just continue to check other branches
                    }
                }
            }
            _ = tokio::time::sleep_until(last_tick + tick_interval) => {
                last_tick = tokio::time::Instant::now();

                // Check for state timeouts
                if client.write().await.check_state_timeout() {
                    error!(target: "net", "Client entered Failed state - shutting down");
                    break LoopExit::Failed;
                }

                // Check if we should attempt reconnection (separate from retry logic)
                if client.write().await.should_reconnect() {
                    let mut client_guard = client.write().await;
                    info!(target: "net", "Reconnection timer expired, attempting reconnection...");
                    if !client_guard.start_reconnection() {
                        info!(target: "net", "Reconnection not available (max attempts or disabled), exiting loop");
                        break LoopExit::ReconnectUnavailable;
                    }
                    // Send initial LoginRequest for reconnection
                    if let Err(e) = client_guard.do_login().await {
                        error!(target: "net", "Failed to send LoginRequest for reconnection: {}", e);
                    }
                    scene_tx.send_replace(client_guard.scene.clone());
                }

                // Check if we should retry in current state. Login retries are
                // opt-in and off by default (`Client::set_login_retry`), so this
                // block is inactive unless explicitly enabled.
                {
                    let mut client_guard = client.write().await;
                    if client_guard.should_retry() {
                        match &client_guard.scene {
                            Scene::Connecting(_connecting) => {
                                info!(target: "net", "Retrying LoginRequest...");
                                if let Err(e) = client_guard.do_login().await {
                                    error!(target: "net", "Failed to send LoginRequest retry: {}", e);
                                }
                                if let Some(connecting) = client_guard.scene.as_connecting_mut() {
                                    connecting.update_retry_time();
                                }
                            }
                            Scene::CharacterSelect(_) => {
                                // In character select, no automatic retry for now
                                // Waiting for character selection from user
                            }
                            Scene::InWorld(_) => {
                                // Already in world, no retry needed
                            }
                            Scene::CharacterCreate(_) => {
                                // Character creation in progress, no retry
                            }
                            Scene::Error(_) => {
                                // Error state - reconnection is handled above, not here
                            }
                        }
                    }
                }

                // Send keepalive if needed
                if last_keepalive.elapsed() >= keepalive_interval {
                    if let Err(e) = client.write().await.send_keepalive().await {
                        error!(target: "net", "Failed to send keep-alive: {}", e);
                    }
                    last_keepalive = tokio::time::Instant::now();
                }
            }
            _ = shutdown_rx.changed() => {
                info!(target: "net", "Client task received shutdown signal");
                break LoopExit::Shutdown;
            }
        }

        // Publish a scene snapshot for observers. `send_replace` is
        // synchronous and non-blocking, so this adds no await while the client
        // lock is held.
        scene_tx.send_replace(client.read().await.scene.clone());
    };

    // Publish the scene the client actually stopped in. Without this, a loop
    // that breaks out of `check_state_timeout` would leave observers watching a
    // stale scene, because the publication above is skipped on every `break`.
    scene_tx.send_replace(client.read().await.scene.clone());
    // Recorded last, so anything that observes an exit reason is guaranteed to
    // be able to read the final scene as well.
    set_exit(&exit, reason.clone());

    let client_id = client.read().await.client_id();
    info!(target: "net", "Client {} network loop stopped ({:?})", client_id, reason);
    info!(target: "net", "Client task shutting down - cleaning up network connections...");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_exit_reason_wins() {
        let (tx, rx) = watch::channel(None);

        set_exit(&tx, LoopExit::LoginRequestFailed);
        set_exit(&tx, LoopExit::Failed);

        assert_eq!(rx.borrow().clone(), Some(LoopExit::LoginRequestFailed));
    }

    /// The fail-safe that keeps `wait_for_exit` from hanging on a panic.
    #[test]
    fn exit_guard_reports_an_unwind() {
        let (tx, rx) = watch::channel(None);

        // Dropping the guard without recording a reason models `run` unwinding.
        drop(ExitGuard { exit: tx.clone() });

        assert!(matches!(rx.borrow().clone(), Some(LoopExit::TaskFailed(_))));
    }

    #[test]
    fn exit_guard_does_not_override_a_recorded_reason() {
        let (tx, rx) = watch::channel(None);
        set_exit(&tx, LoopExit::Shutdown);

        drop(ExitGuard { exit: tx.clone() });

        assert_eq!(rx.borrow().clone(), Some(LoopExit::Shutdown));
    }
}
