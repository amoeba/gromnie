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
use tokio::sync::{RwLock, watch};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::client::Client;
use crate::client::command::{ClientCommand, ClientLoopStopped, ClientSender, LoopExit};
use crate::client::scene::Scene;

/// Handle to a running client driver loop.
///
/// Obtained from [`spawn_client_loop`]. Dropping the handle does not stop the
/// loop; call [`ClientHandle::shutdown`] or send [`ClientCommand::Disconnect`].
pub struct ClientHandle {
    sender: ClientSender,
    scene: watch::Receiver<Scene>,
    exit: watch::Receiver<Option<LoopExit>>,
    shutdown: watch::Sender<bool>,
    join: JoinHandle<()>,
}

impl ClientHandle {
    /// The channels this loop accepts actions on, for handing to a UI, a bot,
    /// or a script that should outlive this handle.
    pub fn sender(&self) -> ClientSender {
        self.sender.clone()
    }

    /// Send a session-level command. Never blocks and never takes the client
    /// lock, so it is safe to call from any task.
    pub fn send(&self, command: ClientCommand) -> Result<(), ClientLoopStopped> {
        self.sender.send(command)
    }

    /// Send a game action directly.
    ///
    /// Prefer this over [`ClientHandle::send`] for anything expressible as a
    /// `GameActionMessage` (chat, movement, trade, magic).
    pub fn send_game_action(&self, action: GameActionMessage) -> Result<(), ClientLoopStopped> {
        self.sender.send_game_action(action)
    }

    /// Subscribe to scene snapshots. Updated at the end of every loop
    /// iteration, so it can lag the client by up to one iteration.
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
/// `initial_login_delay` is an optional caller-requested pause before the first
/// `LoginRequest`. Protocol responses are processed immediately as they arrive.
pub async fn spawn_client_loop(
    client: Arc<RwLock<Client>>,
    initial_login_delay: std::time::Duration,
) -> ClientHandle {
    // Read the sender, the initial snapshot, and claim the client once, before
    // the loop starts contending for the lock.
    let (sender, initial_scene, claimed) = {
        let mut guard = client.write().await;
        (guard.sender(), guard.scene.clone(), guard.claim_driver())
    };

    let (scene_tx, scene_rx) = watch::channel(initial_scene);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (exit_tx, exit_rx) = watch::channel(None);

    let join = tokio::spawn(async move {
        run(
            client,
            claimed,
            scene_tx,
            exit_tx,
            shutdown_rx,
            initial_login_delay,
        )
        .await;
    });

    ClientHandle {
        sender,
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
fn apply_commands(client: &mut Client) -> Option<LoopExit> {
    client.drain_commands()
}

async fn run(
    client: Arc<RwLock<Client>>,
    claimed: bool,
    scene_tx: watch::Sender<Scene>,
    exit: watch::Sender<Option<LoopExit>>,
    mut shutdown_rx: watch::Receiver<bool>,
    initial_login_delay: std::time::Duration,
) {
    // Armed before anything can fail, so an early panic still unblocks anyone
    // waiting on the exit reason.
    let _exit_guard = ExitGuard { exit: exit.clone() };

    // Two loops draining one client would interleave their commands and both
    // think they own the socket, so the second claimer refuses instead.
    if !claimed {
        warn!(
            target: "net",
            "spawn_client_loop called twice for the same client; refusing to drive it again"
        );
        set_exit(
            &exit,
            LoopExit::TaskFailed("client is already being driven".to_string()),
        );
        return;
    }

    let client_id = client.read().await.client_id();
    info!(target: "net", "Client {} network loop started", client_id);

    if !initial_login_delay.is_zero() {
        // Preserve an explicit caller-requested delay, if any.
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

    // Check for timeouts and retries frequently. Packet processing is driven
    // directly by recv_packet, independent of this maintenance tick.
    let tick_interval = tokio::time::Duration::from_millis(10);
    let mut last_tick = tokio::time::Instant::now();

    // `break` carries the reason out so the final scene is published and the exit
    // reason recorded exactly once, in a deterministic order.
    let reason = loop {
        // Apply queued session commands before servicing the socket, so a
        // command issued between iterations takes effect on this one.
        {
            let mut guard = client.write().await;
            if let Some(reason) = apply_commands(&mut guard) {
                break reason;
            }
        }

        tokio::select! {
            // Add a timeout to transport recv so we can respond to shutdown signals
            recv_result = async {
                let mut client_guard = client.write().await;
                tokio::time::timeout(tokio::time::Duration::from_millis(10), client_guard.recv_packet(&mut buf)).await
            } => {
                match recv_result {
                    Ok(Ok((size, peer))) => {
                        let mut client_guard = client.write().await;
                        client_guard.process_packet(&buf[..size], size, &peer).await;

                        if client_guard.has_messages() {
                            client_guard.process_messages();
                        }

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
