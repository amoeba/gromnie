use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

use crate::event_consumer::EventConsumer;
use gromnie_client::client::refactored_event_bus::EventEnvelope;
use gromnie_client::client::Client;

/// Configuration for running a client
pub struct ClientConfig {
    pub id: u32,
    pub address: String,
    pub account_name: String,
    pub password: String,
}

/// Run the client with the provided configuration and event consumer factory
///
/// This encapsulates the common client loop logic:
/// - Creates and connects the client
/// - Spawns event handler task
/// - Runs the network loop with keepalive
/// - Handles graceful shutdown
///
/// The event_consumer_factory is called with the action_tx channel after the client is created.
pub async fn run_client<C, F>(
    config: ClientConfig,
    event_consumer_factory: F,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) where
    C: EventConsumer,
    F: FnOnce(mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>) -> C,
{
    // Create event bus for this client
    let (event_bus, event_rx) = gromnie_client::client::refactored_event_bus::EventBus::new(100);
    let event_sender = event_bus.create_sender(config.id);
    
    let (client, action_tx) = Client::new(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        event_sender,
    )
    .await;

    // Create the event consumer with the action_tx
    let event_consumer = event_consumer_factory(action_tx);

    // Run the client with the event consumer
    run_client_internal(client, event_rx, event_consumer, shutdown_rx).await;
}

/// Run the client with multiple event consumers (event bus pattern)
///
/// This allows registering multiple independent event consumers that each receive
/// all events from the client's event broadcast channel.
pub async fn run_client_with_consumers<F>(
    config: ClientConfig,
    consumers_factory: F,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) where
    F: FnOnce(
        mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>,
    ) -> Vec<Box<dyn EventConsumer>>,
{
    // Create event bus for this client
    let (event_bus, event_rx) = gromnie_client::client::refactored_event_bus::EventBus::new(100);
    let event_sender = event_bus.create_sender(config.id);
    
    let (client, action_tx) = Client::new(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        event_sender,
    )
    .await;

    // Create all event consumers
    let mut consumers = consumers_factory(action_tx);

    // Spawn a task for each consumer with its own event receiver
    let mut consumer_tasks = Vec::new();
    for (idx, mut consumer) in consumers.drain(..).enumerate() {
        let mut consumer_rx = event_bus.subscribe();
        let handle = tokio::spawn(async move {
            info!(target: "events", "Event consumer {} started", idx);

            loop {
                match consumer_rx.recv().await {
                    Ok(event) => {
                        consumer.handle_event(event);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        error!(target: "events", "Consumer {} lagged, {} messages were skipped", idx, skipped);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!(target: "events", "Event channel closed for consumer {}", idx);
                        break;
                    }
                }
            }

            info!(target: "events", "Event consumer {} stopped", idx);
        });
        consumer_tasks.push(handle);
    }

    // Drop the original event_rx since we're not using it
    drop(event_rx);

    // Run the main client loop without event handling (consumers handle events)
    run_client_loop(client, shutdown_rx).await;

    // Wait for consumer tasks to finish
    info!(target: "events", "Waiting for {} consumer tasks to finish", consumer_tasks.len());
    for task in consumer_tasks {
        let _ = task.await;
    }
}

/// Run the client and also send the action_tx channel back to the caller
///
/// This variant is useful for the TUI version where the app needs the action_tx
/// to send commands to the client.
pub async fn run_client_with_action_channel<C, F>(
    config: ClientConfig,
    event_consumer_factory: F,
    action_tx_sender: mpsc::UnboundedSender<
        mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>,
    >,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) where
    C: EventConsumer,
    F: FnOnce(mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>) -> C,
{
    // Create event bus for this client
    let (event_bus, event_rx) = gromnie_client::client::refactored_event_bus::EventBus::new(100);
    let event_sender = event_bus.create_sender(config.id);
    
    let (client, action_tx) = Client::new(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        event_sender,
    )
    .await;

    // Send the action_tx channel back to the caller (e.g., TUI)
    let _ = action_tx_sender.send(action_tx.clone());

    // Create the event consumer with the action_tx
    let event_consumer = event_consumer_factory(action_tx);

    // Run the client with the event consumer
    run_client_internal(client, event_rx, event_consumer, Some(shutdown_rx)).await;
}

/// Internal client runner implementation
async fn run_client_internal<C: EventConsumer>(
    client: Client,
    mut event_rx: broadcast::Receiver<EventEnvelope>,
    mut event_consumer: C,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    // Spawn event handler task
    let event_task = tokio::spawn(async move {
        info!(target: "events", "Event handler task started");

        loop {
            match event_rx.recv().await {
                Ok(envelope) => {
                    // Extract the GameEvent from the envelope for backward compatibility
                    if let Some(game_event) = envelope.extract_game_event() {
                        event_consumer.handle_event(envelope);
                    } else {
                        // For non-game events, we could add logging or other handling
                        debug!(target: "events", "Received non-game event: {:?}", envelope.event);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    error!(target: "events", "Event receiver lagged, {} messages were skipped", skipped);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!(target: "events", "Event channel closed");
                    break;
                }
            }
        }

        info!(target: "events", "Event handler task stopped");
    });

    // Run the main client loop
    run_client_loop(client, shutdown_rx).await;

    // Wait for event handler task to finish
    info!(target: "events", "Waiting for event handler task to finish");
    let _ = event_task.await;
}

/// Main client network loop
async fn run_client_loop(
    mut client: Client,
    mut shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    // Note: We don't call client.connect() here anymore - the client starts in Connecting state
    // and we handle retries in the main loop below

    // Wait before sending initial LoginRequest (to make UI progress visible)
    tokio::time::sleep(tokio::time::Duration::from_millis(
        gromnie_client::client::UI_DELAY_MS,
    ))
    .await;

    // Send initial LoginRequest
    if let Err(e) = client.do_login().await {
        error!("Failed to send initial LoginRequest: {}", e);
        panic!("Failed to send initial LoginRequest");
    }
    info!("Initial LoginRequest sent - entering state machine loop");

    // Main network loop
    let mut buf = [0u8; 1024];
    let mut last_keepalive = tokio::time::Instant::now();
    // Send keepalive every 5 seconds to stay well within the server's timeout window
    // (Server timeout is configurable but defaults to 60s for gameplay, could be as low as 10s)
    let keepalive_interval = tokio::time::Duration::from_secs(5);

    // Tick interval for checking retries and timeouts
    let tick_interval = tokio::time::Duration::from_millis(16); // Check every 16ms (~60 FPS)
    let mut last_tick = tokio::time::Instant::now();

    loop {
        tokio::select! {
            recv_result = client.socket.recv_from(&mut buf) => {
                match recv_result {
                    Ok((size, peer)) => {
                        client.process_packet(&buf[..size], size, &peer).await;

                        if client.has_messages() {
                            client.process_messages();
                        }

                        client.process_actions();

                        if client.has_pending_outgoing_messages()
                            && let Err(e) = client.send_pending_messages().await {
                            error!("Failed to send pending messages: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error in receive loop: {}", e);
                        break;
                    }
                }
            }
            _ = tokio::time::sleep_until(last_tick + tick_interval) => {
                last_tick = tokio::time::Instant::now();

                // Check for state timeouts
                if client.check_state_timeout() {
                    error!("Client entered Failed state - shutting down");
                    break;
                }

                // Check if we should retry in current state
                if client.should_retry() {
                    use gromnie_client::client::ClientState;
                    match client.get_state() {
                        ClientState::Connecting { .. } => {
                            info!("Retrying LoginRequest...");
                            if let Err(e) = client.do_login().await {
                                error!("Failed to send LoginRequest retry: {}", e);
                            }
                            client.update_retry_time();
                        }
                        ClientState::Patching { progress, .. } => {
                            // Only retry if we've already sent the DDD response and are waiting for character list
                            // (i.e., we're in DDDResponseSent state)
                            if matches!(progress, gromnie_client::client::PatchingProgress::DDDResponseSent)
                                && client.get_ddd_response().is_some()
                            {
                                info!("Retrying DDDInterrogationResponse...");
                                if let Err(e) = client.retry_ddd_response().await {
                                    error!("Failed to retry DDDInterrogationResponse: {}", e);
                                }
                            }
                            // If we're still waiting for DDDInterrogation, just wait (no retry)
                            client.update_retry_time();
                        }
                        _ => {}
                    }
                }

                // Send keepalive if needed
                if last_keepalive.elapsed() >= keepalive_interval {
                    if let Err(e) = client.send_keepalive().await {
                        error!("Failed to send keep-alive: {}", e);
                    }
                    last_keepalive = tokio::time::Instant::now();
                }
            }
            _ = async {
                if let Some(ref mut rx) = shutdown_rx {
                    rx.changed().await
                } else {
                    std::future::pending().await
                }
            } => {
                info!("Client task received shutdown signal");
                break;
            }
            _ = tokio::signal::ctrl_c(), if shutdown_rx.is_none() => {
                info!("Received Ctrl+C, shutting down gracefully...");
                break;
            }
        }
    }

    info!("Client task shutting down - cleaning up network connections...");
}
