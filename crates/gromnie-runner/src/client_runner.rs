use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::{error, info};

use crate::event_bus::{EventBus, EventEnvelope};
use crate::event_consumer::EventConsumer;
use crate::event_wrapper::EventWrapper;
use gromnie_client::client::Client;

// Re-export ClientConfig from gromnie-client
pub use gromnie_client::config::ClientConfig;

/// Configuration for running clients - either single or multi-client
#[derive(Clone, Debug)]
pub enum RunConfig {
    /// Run a single client
    Single {
        /// Client configuration
        client: ClientConfig,
    },
    /// Run multiple clients
    Multi {
        /// Base server address (e.g., "localhost:9000")
        server_address: String,
        /// Number of clients to spawn
        num_clients: u32,
        /// Delay between spawning each client (milliseconds)
        spawn_interval_ms: u64,
        /// If true, all clients share a single event bus
        shared_event_bus: bool,
    },
}

impl RunConfig {
    /// Create a single-client run config
    pub fn single(client: ClientConfig) -> Self {
        Self::Single { client }
    }

    /// Create a multi-client run config
    pub fn multi(
        server_address: String,
        num_clients: u32,
        spawn_interval_ms: u64,
        shared_event_bus: bool,
    ) -> Self {
        Self::Multi {
            server_address,
            num_clients,
            spawn_interval_ms,
            shared_event_bus,
        }
    }

    /// Create a multi-client run config with defaults
    pub fn multi_default(server_address: String, num_clients: u32) -> Self {
        Self::Multi {
            server_address,
            num_clients,
            spawn_interval_ms: 1000,
            shared_event_bus: false,
        }
    }
}

/// Configuration for running multiple clients
#[derive(Clone, Debug)]
pub struct MultiClientConfig {
    /// Base server address (e.g., "localhost:9000")
    pub server_address: String,

    /// Number of clients to spawn
    pub num_clients: u32,

    /// Delay between spawning each client (milliseconds)
    pub spawn_interval_ms: u64,

    /// If true, all clients share a single event bus
    /// If false, each client gets its own event bus
    pub shared_event_bus: bool,
}

impl MultiClientConfig {
    /// Create a new multi-client config with defaults
    pub fn new(server_address: String, num_clients: u32) -> Self {
        Self {
            server_address,
            num_clients,
            spawn_interval_ms: 1000,
            shared_event_bus: false,
        }
    }

    /// Set the spawn interval
    pub fn with_spawn_interval(mut self, interval_ms: u64) -> Self {
        self.spawn_interval_ms = interval_ms;
        self
    }

    /// Set whether to use a shared event bus
    pub fn with_shared_event_bus(mut self, shared: bool) -> Self {
        self.shared_event_bus = shared;
        self
    }
}

/// Statistics shared across all clients in a multi-client run
#[derive(Default)]
pub struct MultiClientStats {
    /// Number of clients we attempted to spawn
    pub attempted: AtomicU32,
    /// Number of clients that successfully started
    pub spawned: AtomicU32,
    /// Number of clients that authenticated successfully
    pub authenticated: AtomicU32,
    /// Number of characters created
    pub character_created: AtomicU32,
    /// Number of clients that logged in successfully
    pub logged_in: AtomicU32,
    /// Number of errors encountered
    pub errors: AtomicU32,
    /// Number of client tasks that panicked or failed to start
    pub task_failures: AtomicU32,
}

impl MultiClientStats {
    /// Print statistics to the log
    pub fn print(&self, elapsed_secs: u64) {
        let attempted = self.attempted.load(Ordering::SeqCst);
        let spawned = self.spawned.load(Ordering::SeqCst);
        let auth = self.authenticated.load(Ordering::SeqCst);
        let login = self.logged_in.load(Ordering::SeqCst);
        let char_created = self.character_created.load(Ordering::SeqCst);
        let errors = self.errors.load(Ordering::SeqCst);
        let task_failures = self.task_failures.load(Ordering::SeqCst);

        info!(
            "[Stats @ {}s] Attempted: {} | Spawned: {} | Auth: {} | LoggedIn: {} | CharCreated: {} | Errors: {} | TaskFailures: {}",
            elapsed_secs, attempted, spawned, auth, login, char_created, errors, task_failures
        );
    }

    /// Print final statistics
    pub fn print_final(&self, total_time: Duration) {
        let attempted = self.attempted.load(Ordering::SeqCst);
        let spawned = self.spawned.load(Ordering::SeqCst);
        let auth = self.authenticated.load(Ordering::SeqCst);
        let login = self.logged_in.load(Ordering::SeqCst);
        let errors = self.errors.load(Ordering::SeqCst);
        let task_failures = self.task_failures.load(Ordering::SeqCst);

        info!("========================================");
        info!("Multi-client run complete");
        info!("Total time: {:.2}s", total_time.as_secs_f64());
        info!("========================================");
        info!("Client Launch:");
        info!("  Attempted:      {}", attempted);
        info!("  Spawned:        {}", spawned);
        info!("  Task failures:  {}", task_failures);
        info!(
            "  Failed to spawn: {}",
            attempted.saturating_sub(spawned + task_failures)
        );
        info!("========================================");
        info!("Client Progress:");
        info!("  Authenticated:  {}", auth);
        info!("  Logged in:      {}", login);
        info!("  Auth/login errors: {}", errors);
        info!("========================================");
        if attempted > 0 {
            info!(
                "Success rate: {:.1}% ({} / {} attempted)",
                (login as f64 / attempted as f64) * 100.0,
                login,
                attempted
            );
        }
        info!("========================================");
    }
}

/// Factory for creating consumers for each client in a multi-client run
///
/// This trait allows the caller to customize which consumers are created
/// for each client, while the runner handles the orchestration.
pub trait MultiClientConsumerFactory: Send + Sync {
    /// Create a consumer for a specific client
    fn create_consumer(
        &self,
        client_id: u32,
        client_config: &ClientConfig,
        action_tx: mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Box<dyn EventConsumer>;
}

/// Simple adapter to convert a closure into a MultiClientConsumerFactory
pub struct FnConsumerFactory<F>
where
    F: Fn(
            u32,
            &ClientConfig,
            mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
        ) -> Box<dyn EventConsumer>
        + Send
        + Sync,
{
    f: F,
}

impl<F> FnConsumerFactory<F>
where
    F: Fn(
            u32,
            &ClientConfig,
            mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
        ) -> Box<dyn EventConsumer>
        + Send
        + Sync,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> MultiClientConsumerFactory for FnConsumerFactory<F>
where
    F: Fn(
            u32,
            &ClientConfig,
            mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
        ) -> Box<dyn EventConsumer>
        + Send
        + Sync,
{
    fn create_consumer(
        &self,
        client_id: u32,
        client_config: &ClientConfig,
        action_tx: mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Box<dyn EventConsumer> {
        (self.f)(client_id, client_config, action_tx)
    }
}

/// Shared event bus manager that owns the central event bus
#[derive(Clone)]
pub struct EventBusManager {
    pub(crate) event_bus: Arc<EventBus>,
}

impl EventBusManager {
    /// Create a new event bus manager
    pub fn new(capacity: usize) -> Self {
        let (event_bus, _) = EventBus::new(capacity);
        Self {
            event_bus: Arc::new(event_bus),
        }
    }

    /// Create an event sender for a specific client
    pub fn create_sender(&self, client_id: u32) -> crate::event_bus::EventSender {
        self.event_bus.create_sender(client_id)
    }

    /// Create a new event subscriber
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.event_bus.subscribe()
    }

    /// Get the number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.event_bus.subscriber_count()
    }
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
    event_bus_manager: Arc<EventBusManager>,
    event_consumer_factory: F,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) where
    C: EventConsumer,
    F: FnOnce(mpsc::UnboundedSender<gromnie_events::SimpleClientAction>) -> C,
{
    // Create a channel for raw events from client to EventWrapper
    let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

    // Spawn EventWrapper to bridge client events to event bus
    let event_wrapper = EventWrapper::new(config.id, event_bus_manager.event_bus.clone());
    tokio::spawn(async move {
        event_wrapper.run(raw_event_rx).await;
    });

    // Subscribe to the event bus for the consumer
    let event_rx = event_bus_manager.subscribe();

    let (client, action_tx) = Client::new_with_reconnect(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        config.character_name.clone(),
        raw_event_tx,
        config.reconnect.clone(),
    )
    .await;

    // Create the event consumer with the action_tx
    let event_consumer = event_consumer_factory(action_tx);

    // Run the client with the event consumer
    run_client_internal(client, event_rx, Box::new(event_consumer), shutdown_rx).await;
}

/// Run the client with multiple event consumers (event bus pattern)
///
/// This allows registering multiple independent event consumers that each receive
/// all events from the client's event broadcast channel.
pub async fn run_client_with_consumers<F>(
    config: ClientConfig,
    event_bus_manager: Arc<EventBusManager>,
    consumers_factory: F,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) where
    F: FnOnce(
        mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Vec<Box<dyn EventConsumer>>,
{
    use crate::event_bus::{EventEnvelope, EventSource, EventType, SystemEvent};

    // Create a channel for raw events from client to EventWrapper
    let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

    // Spawn EventWrapper to bridge client events to event bus
    let event_wrapper = EventWrapper::new(config.id, event_bus_manager.event_bus.clone());
    tokio::spawn(async move {
        event_wrapper.run(raw_event_rx).await;
    });

    let (client, action_tx) = Client::new_with_reconnect(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        config.character_name.clone(),
        raw_event_tx,
        config.reconnect.clone(),
    )
    .await;

    // Create all event consumers
    let mut consumers = consumers_factory(action_tx);

    // Create a shutdown sender that will be used to signal consumers to stop
    let shutdown_sender = event_bus_manager.create_sender(config.id);

    // Spawn a task for each consumer with its own event receiver
    let mut consumer_tasks = Vec::new();
    for (idx, mut consumer) in consumers.drain(..).enumerate() {
        let mut consumer_rx = event_bus_manager.subscribe();
        let handle = tokio::spawn(async move {
            info!(target: "events", "Event consumer {} started", idx);

            loop {
                match consumer_rx.recv().await {
                    Ok(envelope) => {
                        // Check if this is a shutdown event
                        if matches!(&envelope.event, EventType::System(SystemEvent::Shutdown)) {
                            info!(target: "events", "Event consumer {} received shutdown signal", idx);
                            break;
                        }
                        consumer.handle_event(envelope);
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

    // Run the main client loop without event handling (consumers handle events)
    run_client_loop(client, shutdown_rx).await;

    // Send shutdown event to all consumers
    info!(target: "events", "Sending shutdown signal to consumers");
    let shutdown_event =
        EventEnvelope::system_event(SystemEvent::Shutdown, config.id, 0, EventSource::System);
    shutdown_sender.publish(shutdown_event);

    // Give consumers a moment to receive and process the shutdown event
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
    event_bus_manager: Arc<EventBusManager>,
    event_consumer_factory: F,
    action_tx_sender: mpsc::UnboundedSender<
        mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    >,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) where
    C: EventConsumer,
    F: FnOnce(mpsc::UnboundedSender<gromnie_events::SimpleClientAction>) -> C,
{
    // Create a channel for raw events from client to EventWrapper
    let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

    // Spawn EventWrapper to bridge client events to event bus
    let event_wrapper = EventWrapper::new(config.id, event_bus_manager.event_bus.clone());
    tokio::spawn(async move {
        event_wrapper.run(raw_event_rx).await;
    });

    // Subscribe to the event bus for the consumer
    let event_rx = event_bus_manager.subscribe();

    let (client, action_tx) = Client::new_with_reconnect(
        config.id,
        config.address.clone(),
        config.account_name.clone(),
        config.password.clone(),
        config.character_name.clone(),
        raw_event_tx,
        config.reconnect.clone(),
    )
    .await;

    // Send the action_tx channel back to the caller (e.g., TUI)
    let _ = action_tx_sender.send(action_tx.clone());

    // Create the event consumer with the action_tx
    let event_consumer = event_consumer_factory(action_tx);

    // Run the client with the event consumer
    run_client_internal(
        client,
        event_rx,
        Box::new(event_consumer),
        Some(shutdown_rx),
    )
    .await;
}

/// Internal client runner implementation
pub(crate) async fn run_client_internal(
    client: Client,
    mut event_rx: broadcast::Receiver<EventEnvelope>,
    mut event_consumer: Box<dyn EventConsumer>,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    // Spawn event handler task
    let event_task = tokio::spawn(async move {
        info!(target: "events", "Event handler task started");

        loop {
            match event_rx.recv().await {
                Ok(envelope) => {
                    event_consumer.handle_event(envelope);
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

    // Wait for event handler task to finish with a timeout
    info!(target: "events", "Waiting for event handler task to finish");
    let timeout = tokio::time::Duration::from_secs(1);
    match tokio::time::timeout(timeout, event_task).await {
        Ok(Ok(())) => {
            info!(target: "events", "Event handler task finished gracefully");
        }
        Ok(Err(e)) => {
            error!(target: "events", "Event handler task panicked: {}", e);
        }
        Err(_) => {
            info!(target: "events", "Event handler task did not finish within timeout, continuing shutdown");
        }
    }
}

/// Run the main client network loop
async fn run_client_loop(
    mut client: Client,
    mut shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    use tracing::error;

    info!(target: "net", "Client {} network loop started", client.client_id());

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
    let tick_interval = tokio::time::Duration::from_millis(100); // Check every 100ms
    let mut last_tick = tokio::time::Instant::now();

    loop {
        tokio::select! {
            // Add a timeout to recv_from so we can respond to shutdown signals
            recv_result = tokio::time::timeout(tokio::time::Duration::from_millis(100), client.socket.recv_from(&mut buf)) => {
                match recv_result {
                    Ok(Ok((size, peer))) => {
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
                    Ok(Err(e)) => {
                        error!("Error in receive loop: {}", e);
                        // Always transition to disconnected state on socket error
                        // UDP socket errors are serious and indicate network problems
                        client.enter_disconnected();
                    }
                    Err(_) => {
                        // Timeout - this is normal, just continue to check other branches
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
                        ClientState::Disconnected { .. } => {
                            // Check if we should attempt reconnection
                            if client.should_reconnect() {
                                if !client.start_reconnection() {
                                    info!("Reconnection not available, exiting loop");
                                    break;
                                }
                                // Send initial LoginRequest for reconnection
                                if let Err(e) = client.do_login().await {
                                    error!("Failed to send LoginRequest for reconnection: {}", e);
                                }
                            }
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

    info!("Client {} network loop stopped", client.client_id());
    info!("Client task shutting down - cleaning up network connections...");
}

/// Create a new EventBusManager for managing a shared event bus
pub fn create_event_bus_manager() -> EventBusManager {
    EventBusManager::new(100)
}

/// Run multiple clients with coordinated lifecycle
///
/// This function:
/// - Spawns the specified number of clients with rate limiting
/// - Optionally shares a single event bus across all clients
/// - Collects statistics across all clients
/// - Handles graceful shutdown for all clients
///
/// # Arguments
/// * `config` - Multi-client configuration (server address, num clients, etc.)
/// * `consumer_factory` - Factory for creating consumers per client
/// * `client_config_generator` - Function to generate client config for each client ID
/// * `shutdown_rx` - Optional shutdown signal receiver
///
/// # Returns
/// Statistics collected during the run
pub async fn run_multi_client<G>(
    config: MultiClientConfig,
    consumer_factory: Arc<dyn MultiClientConsumerFactory>,
    client_config_generator: G,
    shutdown_rx: Option<watch::Receiver<bool>>,
) -> Arc<MultiClientStats>
where
    G: Fn(u32) -> ClientConfig + Send + Sync + 'static,
{
    let stats = Arc::new(MultiClientStats::default());
    let (local_shutdown_tx, local_shutdown_rx) = watch::channel(false);

    // Use provided shutdown channel or create our own
    let shutdown_rx = shutdown_rx.unwrap_or(local_shutdown_rx);

    // Create shared event bus if configured
    let shared_event_bus = if config.shared_event_bus {
        Some(Arc::new(EventBusManager::new(100)))
    } else {
        None
    };

    let mut join_handles = vec![];
    let _start_time = Instant::now();

    info!(
        "Starting multi-client run: {} clients to {}",
        config.num_clients, config.server_address
    );
    info!(
        "Rate limiting: {} ms between connections",
        config.spawn_interval_ms
    );

    // Spawn client tasks
    for client_id in 0..config.num_clients {
        stats.attempted.fetch_add(1, Ordering::SeqCst);

        let client_config = client_config_generator(client_id);
        let event_bus_manager = shared_event_bus
            .clone()
            .unwrap_or_else(|| Arc::new(EventBusManager::new(100)));

        let consumer_factory = consumer_factory.clone();
        let stats = stats.clone();
        let mut shutdown_rx = shutdown_rx.clone();
        let spawn_interval = config.spawn_interval_ms;

        let handle = tokio::spawn(async move {
            // Rate limiting: stagger client connections
            let sleep_duration = Duration::from_millis(client_id as u64 * spawn_interval);
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    // Sleep completed normally, proceed with client connection
                }
                _ = shutdown_rx.changed() => {
                    // Shutdown signal received during sleep, exit early
                    return;
                }
            }

            // Create the consumer for this client
            let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

            // Spawn EventWrapper to bridge client events to event bus
            let event_wrapper =
                EventWrapper::new(client_config.id, event_bus_manager.event_bus.clone());
            tokio::spawn(async move {
                event_wrapper.run(raw_event_rx).await;
            });

            // Subscribe to the event bus for the consumer
            let event_rx = event_bus_manager.subscribe();

            // Create the client
            let (client, action_tx) = Client::new(
                client_config.id,
                client_config.address.clone(),
                client_config.account_name.clone(),
                client_config.password.clone(),
                client_config.character_name.clone(),
                raw_event_tx,
            )
            .await;

            // Mark that we successfully spawned this client
            stats.spawned.fetch_add(1, Ordering::SeqCst);

            // Create the event consumer
            let event_consumer =
                consumer_factory.create_consumer(client_config.id, &client_config, action_tx);

            // Run the client
            run_client_internal(client, event_rx, event_consumer, Some(shutdown_rx)).await;
        });

        join_handles.push(handle);
    }

    info!("All clients spawned, waiting for events...");

    // Wait for Ctrl+C or external shutdown
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Received Ctrl+C, shutting down all clients...");
            info!("Press Ctrl+C again to force quit");
            let _ = local_shutdown_tx.send(true);
        }
        Err(e) => {
            error!("Failed to listen for Ctrl+C: {}", e);
        }
    }

    // Wait for all client tasks to complete with timeout or second Ctrl+C
    let shutdown_timeout = Duration::from_secs(5);
    let wait_future = async {
        for (idx, handle) in join_handles.into_iter().enumerate() {
            match handle.await {
                Ok(_) => {
                    // Task completed normally
                }
                Err(e) => {
                    // Task panicked or was cancelled
                    stats.task_failures.fetch_add(1, Ordering::SeqCst);
                    if e.is_panic() {
                        error!("[Client {}] Task panicked: {:?}", idx, e);
                    } else {
                        error!("[Client {}] Task was cancelled", idx);
                    }
                }
            }
        }
    };

    tokio::select! {
        _ = wait_future => {
            info!("All clients shut down gracefully");
        }
        _ = tokio::time::sleep(shutdown_timeout) => {
            info!("Shutdown timeout reached, forcing exit");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Second Ctrl+C received, forcing immediate exit");
        }
    }

    stats
}

/// Result from running clients
pub enum RunResult {
    /// Single client completed
    Single,
    /// Multi-client run completed with statistics
    Multi(Arc<MultiClientStats>),
}

/// Builder for creating consumers for each client
///
/// This trait provides a flexible way to create consumers for each client,
/// whether single or multi-client mode.
pub trait ConsumerBuilder: Send + Sync {
    /// Create a consumer for a single client
    fn build(
        &self,
        client_id: u32,
        client_config: &ClientConfig,
        action_tx: mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Box<dyn EventConsumer>;
}

/// Type alias for consumer factory closure
type ConsumerFactoryFn = dyn Fn(
        u32,
        &ClientConfig,
        mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Box<dyn EventConsumer>
    + Send
    + Sync;

/// Adapter for closure-based consumer builders
pub struct FnConsumerBuilder {
    f: Box<ConsumerFactoryFn>,
}

impl FnConsumerBuilder {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(
                u32,
                &ClientConfig,
                mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
            ) -> Box<dyn EventConsumer>
            + Send
            + Sync
            + 'static,
    {
        Self { f: Box::new(f) }
    }
}

impl ConsumerBuilder for FnConsumerBuilder {
    fn build(
        &self,
        client_id: u32,
        client_config: &ClientConfig,
        action_tx: mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
    ) -> Box<dyn EventConsumer> {
        (self.f)(client_id, client_config, action_tx)
    }
}

/// Unified client runner
///
/// This function handles both single-client and multi-client runs based on the config.
///
/// # Arguments
/// * `config` - Run configuration (single or multi-client)
/// * `consumer_builder` - Builder for creating consumers per client
/// * `client_config_generator` - Optional function to generate client config for each client ID (multi-client only)
/// * `shutdown_rx` - Optional shutdown signal receiver
///
/// # Returns
/// Run result with statistics for multi-client runs
pub async fn run<B, G>(
    config: RunConfig,
    consumer_builder: B,
    client_config_generator: Option<G>,
    shutdown_rx: Option<watch::Receiver<bool>>,
) -> RunResult
where
    B: ConsumerBuilder + 'static,
    G: Fn(u32) -> ClientConfig + Send + Sync + 'static,
{
    match config {
        RunConfig::Single { client } => {
            let event_bus_manager = Arc::new(EventBusManager::new(100));

            let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

            let event_wrapper = EventWrapper::new(client.id, event_bus_manager.event_bus.clone());
            tokio::spawn(async move {
                event_wrapper.run(raw_event_rx).await;
            });

            let event_rx = event_bus_manager.subscribe();

            let (client_obj, action_tx) = Client::new(
                client.id,
                client.address.clone(),
                client.account_name.clone(),
                client.password.clone(),
                client.character_name.clone(),
                raw_event_tx,
            )
            .await;

            let event_consumer = consumer_builder.build(client.id, &client, action_tx);

            run_client_internal(client_obj, event_rx, event_consumer, shutdown_rx).await;

            RunResult::Single
        }
        RunConfig::Multi {
            server_address,
            num_clients,
            spawn_interval_ms,
            shared_event_bus,
        } => {
            let multi_config = MultiClientConfig {
                server_address,
                num_clients,
                spawn_interval_ms,
                shared_event_bus,
            };

            // Create an adapter from ConsumerBuilder to MultiClientConsumerFactory
            struct Adapter<T> {
                inner: T,
            }

            impl<T: ConsumerBuilder> MultiClientConsumerFactory for Adapter<T> {
                fn create_consumer(
                    &self,
                    client_id: u32,
                    client_config: &ClientConfig,
                    action_tx: mpsc::UnboundedSender<gromnie_events::SimpleClientAction>,
                ) -> Box<dyn EventConsumer> {
                    self.inner.build(client_id, client_config, action_tx)
                }
            }

            let factory = Adapter {
                inner: consumer_builder,
            };

            let stats = match client_config_generator {
                Some(config_gen) => {
                    run_multi_client(multi_config, Arc::new(factory), config_gen, shutdown_rx).await
                }
                None => {
                    let default_gen = |id| {
                        ClientConfig::new(
                            id,
                            "localhost:9000".to_string(),
                            format!("client_{}", id),
                            format!("client_{}", id),
                        )
                        .with_reconnect(Default::default())
                    };
                    run_multi_client(multi_config, Arc::new(factory), default_gen, shutdown_rx)
                        .await
                }
            };

            RunResult::Multi(stats)
        }
    }
}
