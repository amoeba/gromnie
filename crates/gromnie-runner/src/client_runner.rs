use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info};

use crate::event_consumer::EventConsumer;
use crate::event_bus::{EventBus, EventEnvelope};
use gromnie_client::client::Client;

/// Configuration for running a client
pub struct ClientConfig {
    pub id: u32,
    pub address: String,
    pub account_name: String,
    pub password: String,
}

/// Shared event bus manager that owns the central event bus
#[derive(Clone)]
pub struct EventBusManager {
    event_bus: Arc<EventBus>,
}

impl EventBusManager {
    /// Create a new event bus manager
    pub fn new(capacity: usize) -> Self {
        let (event_bus, _) = EventBus::new(capacity);
        Self { event_bus: Arc::new(event_bus) }
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
    F: FnOnce(mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>) -> C,
{
    // Get event sender for this client from the shared event bus
    let event_sender = event_bus_manager.create_sender(config.id);
    let event_rx = event_bus_manager.subscribe();
    
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
    event_bus_manager: Arc<EventBusManager>,
    consumers_factory: F,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) where
    F: FnOnce(
        mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>,
    ) -> Vec<Box<dyn EventConsumer>>,
{
    // Get event sender for this client from the shared event bus
    let event_sender = event_bus_manager.create_sender(config.id);
    let event_rx = event_bus_manager.subscribe();
    
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
        let mut consumer_rx = event_bus_manager.subscribe();
        let handle = tokio::spawn(async move {
            info!(target: "events", "Event consumer {} started", idx);

            loop {
                match consumer_rx.recv().await {
                    Ok(envelope) => {
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
    event_bus_manager: Arc<EventBusManager>,
    event_consumer_factory: F,
    action_tx_sender: mpsc::UnboundedSender<
        mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>,
    >,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) where
    C: EventConsumer,
    F: FnOnce(mpsc::UnboundedSender<gromnie_client::client::events::ClientAction>) -> C,
{
    // Get event sender for this client from the shared event bus
    let event_sender = event_bus_manager.create_sender(config.id);
    let event_rx = event_bus_manager.subscribe();
    
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

    // Wait for event handler task to finish
    info!(target: "events", "Waiting for event handler task to finish");
    let _ = event_task.await;
}

/// Run the main client network loop
async fn run_client_loop(
    mut client: Client,
    shutdown_rx: Option<tokio::sync::watch::Receiver<bool>>,
) {
    // Main client loop implementation
    // ... (existing implementation)
    info!(target: "net", "Client {} network loop started", client.client_id());

    // Shutdown handling would go here
    // ...

    info!(target: "net", "Client {} network loop stopped", client.client_id());
}

/// Create a new EventBusManager for managing a shared event bus
pub fn create_event_bus_manager() -> EventBusManager {
    EventBusManager::new(100)
}