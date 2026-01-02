//! Builder pattern for ClientRunner
//!
//! This module provides a clean, ergonomic API for configuring and running game clients.

use std::sync::Arc;
use tokio::sync::{mpsc, watch};

use crate::event_consumer::EventConsumer;
use gromnie_client::config::scripting_config::ScriptingConfig;
use gromnie_events::SimpleClientAction;

// Re-export types
pub use gromnie_client::config::ClientConfig;
pub use gromnie_events::{ConsumerContext, ConsumerFactory};

/// Client mode - either static configs or dynamic generation
pub enum ClientMode {
    /// Run with static client configuration(s)
    Static {
        /// Static list of client configurations
        configs: Vec<ClientConfig>,
        /// Delay between spawning each client (milliseconds)
        spawn_interval_ms: u64,
        /// If true, all clients share a single event bus
        shared_event_bus: bool,
    },
    /// Run multiple clients with dynamic configuration generation
    Dynamic {
        /// Number of clients to spawn
        num_clients: u32,
        /// Server address for all clients
        server_address: String,
        /// Delay between spawning each client (milliseconds)
        spawn_interval_ms: u64,
        /// If true, all clients share a single event bus
        shared_event_bus: bool,
        /// Function to generate client config for each client ID
        generator: Box<dyn Fn(u32) -> ClientConfig + Send + Sync>,
    },
}

/// Error during builder configuration
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Client mode not specified - use .with_clients() or .with_client_generator()")]
    MissingClientMode,
}

/// Trait for types that can be converted into a vector of client configs
///
/// This allows the API to accept either a single ClientConfig or Vec<ClientConfig>
pub trait IntoClientConfigs {
    fn into_configs(self) -> Vec<ClientConfig>;
}

impl IntoClientConfigs for ClientConfig {
    fn into_configs(self) -> Vec<ClientConfig> {
        vec![self]
    }
}

impl IntoClientConfigs for Vec<ClientConfig> {
    fn into_configs(self) -> Vec<ClientConfig> {
        self
    }
}

/// Builder for ClientRunner
pub struct ClientRunnerBuilder {
    mode: Option<ClientMode>,
    consumers: Vec<Box<dyn ConsumerFactory>>,
    action_channel: Option<mpsc::UnboundedSender<mpsc::UnboundedSender<SimpleClientAction>>>,
    shutdown_rx: Option<watch::Receiver<bool>>,
    event_bus_capacity: usize,
    app_config: Option<gromnie_client::config::GromnieConfig>,
}

impl ClientRunnerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            mode: None,
            consumers: Vec::new(),
            action_channel: None,
            shutdown_rx: None,
            event_bus_capacity: 100,
            app_config: None,
        }
    }

    /// Configure with one or more client configs
    ///
    /// Accepts either a single `ClientConfig` or a `Vec<ClientConfig>`.
    ///
    /// # Examples
    ///
    /// Single client:
    /// ```no_run
    /// # use gromnie_runner::{ClientRunner, ClientConfig};
    /// ClientRunner::builder()
    ///     .with_clients(ClientConfig::new(0, "localhost:9000".into(), "user".into(), "pass".into()));
    /// ```
    ///
    /// Multiple clients:
    /// ```no_run
    /// # use gromnie_runner::{ClientRunner, ClientConfig};
    /// let configs = vec![
    ///     ClientConfig::new(0, "localhost:9000".into(), "user1".into(), "pass1".into()),
    ///     ClientConfig::new(1, "localhost:9000".into(), "user2".into(), "pass2".into()),
    /// ];
    /// ClientRunner::builder()
    ///     .with_clients(configs);
    /// ```
    pub fn with_clients(mut self, configs: impl IntoClientConfigs) -> Self {
        let configs = configs.into_configs();
        let spawn_interval_ms = if configs.len() == 1 { 0 } else { 1000 };

        self.mode = Some(ClientMode::Static {
            configs,
            spawn_interval_ms,
            shared_event_bus: false,
        });
        self
    }

    /// Configure with a client generator for dynamic multi-client mode
    ///
    /// # Arguments
    /// * `num_clients` - Number of clients to spawn
    /// * `server_address` - Server address (e.g., "localhost:9000")
    /// * `generator` - Function to generate ClientConfig for each client ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use gromnie_runner::{ClientRunner, ClientConfig};
    /// ClientRunner::builder()
    ///     .with_client_generator(
    ///         100,
    ///         "localhost:9000".into(),
    ///         |client_id| ClientConfig::new(
    ///             client_id,
    ///             "localhost:9000".into(),
    ///             format!("bot_{}", client_id),
    ///             format!("pass_{}", client_id)
    ///         )
    ///     );
    /// ```
    pub fn with_client_generator<F>(
        mut self,
        num_clients: u32,
        server_address: String,
        generator: F,
    ) -> Self
    where
        F: Fn(u32) -> ClientConfig + Send + Sync + 'static,
    {
        self.mode = Some(ClientMode::Dynamic {
            num_clients,
            server_address,
            spawn_interval_ms: 1000,
            shared_event_bus: false,
            generator: Box::new(generator),
        });
        self
    }

    /// Set the spawn interval for multi-client mode (milliseconds)
    pub fn spawn_interval_ms(mut self, interval_ms: u64) -> Self {
        match &mut self.mode {
            Some(ClientMode::Static {
                spawn_interval_ms, ..
            })
            | Some(ClientMode::Dynamic {
                spawn_interval_ms, ..
            }) => {
                *spawn_interval_ms = interval_ms;
            }
            _ => {
                // Ignore if mode not set yet
            }
        }
        self
    }

    /// Set whether to use a shared event bus in multi-client mode
    pub fn shared_event_bus(mut self, shared: bool) -> Self {
        match &mut self.mode {
            Some(ClientMode::Static {
                shared_event_bus, ..
            })
            | Some(ClientMode::Dynamic {
                shared_event_bus, ..
            }) => {
                *shared_event_bus = shared;
            }
            _ => {
                // Ignore if mode not set yet
            }
        }
        self
    }

    /// Add an event consumer
    pub fn with_consumer<C: ConsumerFactory>(mut self, consumer: C) -> Self {
        self.consumers.push(Box::new(consumer));
        self
    }

    /// Provide a channel to receive the action_tx (useful for TUI)
    pub fn with_action_channel(
        mut self,
        tx: mpsc::UnboundedSender<mpsc::UnboundedSender<SimpleClientAction>>,
    ) -> Self {
        self.action_channel = Some(tx);
        self
    }

    /// Provide a shutdown receiver
    pub fn with_shutdown(mut self, rx: watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(rx);
        self
    }

    /// Set the event bus capacity (default: 100)
    pub fn event_bus_capacity(mut self, capacity: usize) -> Self {
        self.event_bus_capacity = capacity;
        self
    }

    /// Set application config (optional - will load from default location if not specified)
    pub fn with_config(mut self, config: gromnie_client::config::GromnieConfig) -> Self {
        self.app_config = Some(config);
        self
    }

    /// Build the ClientRunner
    ///
    /// If config was not provided via `with_config()`, it will be loaded
    /// from the default location. Scripting will be automatically configured based
    /// on the config settings.
    ///
    /// # Examples
    ///
    /// Single client (config loaded automatically):
    /// ```no_run
    /// # use gromnie_runner::{ClientRunner, ClientConfig, LoggingConsumer};
    /// # async fn example() {
    /// ClientRunner::builder()
    ///     .with_clients(ClientConfig::new(0, "localhost:9000".into(), "user".into(), "pass".into()))
    ///     .with_consumer(LoggingConsumer::from_factory())
    ///     .build()
    ///     .unwrap()
    ///     .run()
    ///     .await;
    /// # }
    /// ```
    ///
    /// Multiple clients with custom config:
    /// ```no_run
    /// # use gromnie_runner::{ClientRunner, ClientConfig, LoggingConsumer};
    /// # use gromnie_client::config::GromnieConfig;
    /// # async fn example(custom_config: GromnieConfig) {
    /// let configs = vec![
    ///     ClientConfig::new(0, "localhost:9000".into(), "user1".into(), "pass1".into()),
    ///     ClientConfig::new(1, "localhost:9000".into(), "user2".into(), "pass2".into()),
    /// ];
    /// ClientRunner::builder()
    ///     .with_clients(configs)
    ///     .with_consumer(LoggingConsumer::from_factory())
    ///     .with_config(custom_config)
    ///     .build()
    ///     .unwrap()
    ///     .run()
    ///     .await;
    /// # }
    /// ```
    pub fn build(mut self) -> Result<ClientRunner, BuildError> {
        let mode = self.mode.ok_or(BuildError::MissingClientMode)?;

        // Load config from default location if not provided
        let config = match self.app_config {
            Some(cfg) => cfg,
            None => {
                // Try to load from default location, or create minimal config if not found
                gromnie_client::config::GromnieConfig::load().unwrap_or_else(|_| {
                    // Create minimal default config with scripting disabled
                    gromnie_client::config::GromnieConfig {
                        servers: std::collections::BTreeMap::new(),
                        accounts: std::collections::BTreeMap::new(),
                        scripting: ScriptingConfig {
                            enabled: false,
                            script_dir: None,
                            config: std::collections::HashMap::new(),
                        },
                    }
                })
            }
        };

        // Setup scripting if enabled in config
        if config.scripting.enabled {
            let scripting_config = config.scripting.clone();
            self.consumers.push(Box::new(move |ctx: &ConsumerContext| {
                let consumer = gromnie_scripting_host::create_script_consumer(
                    ctx.action_tx.clone(),
                    &scripting_config,
                );
                Box::new(consumer) as Box<dyn EventConsumer>
            }));
        }

        Ok(ClientRunner {
            mode,
            consumers: self.consumers,
            action_channel: self.action_channel,
            shutdown_rx: self.shutdown_rx,
            event_bus_capacity: self.event_bus_capacity,
            app_config: Some(config),
        })
    }
}

impl Default for ClientRunnerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configured client runner ready to execute
pub struct ClientRunner {
    pub(crate) mode: ClientMode,
    pub(crate) consumers: Vec<Box<dyn ConsumerFactory>>,
    pub(crate) action_channel:
        Option<mpsc::UnboundedSender<mpsc::UnboundedSender<SimpleClientAction>>>,
    pub(crate) shutdown_rx: Option<watch::Receiver<bool>>,
    pub(crate) event_bus_capacity: usize,
    pub(crate) app_config: Option<gromnie_client::config::GromnieConfig>,
}

/// Result from running clients
pub enum RunResult {
    /// Single client completed
    Single,
    /// Multi-client run completed with statistics
    Multi(Arc<crate::client_runner::MultiClientStats>),
}

impl ClientRunner {
    /// Create a new builder
    pub fn builder() -> ClientRunnerBuilder {
        ClientRunnerBuilder::new()
    }

    /// Run the configured client(s)
    pub async fn run(mut self) -> RunResult {
        // Take ownership of mode to avoid partial move
        let mode = std::mem::replace(
            &mut self.mode,
            ClientMode::Static {
                configs: vec![],
                spawn_interval_ms: 0,
                shared_event_bus: false,
            },
        );

        match mode {
            ClientMode::Static {
                configs,
                spawn_interval_ms,
                shared_event_bus,
            } => {
                if configs.len() == 1 {
                    // Single client - optimize by not using multi-client machinery
                    self.run_single(configs.into_iter().next().unwrap()).await
                } else {
                    // Multiple static clients
                    self.run_static(configs, spawn_interval_ms, shared_event_bus)
                        .await
                }
            }
            ClientMode::Dynamic {
                num_clients,
                server_address,
                spawn_interval_ms,
                shared_event_bus,
                generator,
            } => {
                self.run_dynamic(
                    num_clients,
                    server_address,
                    spawn_interval_ms,
                    shared_event_bus,
                    generator,
                )
                .await
            }
        }
    }

    /// Run a single client (internal)
    async fn run_single(self, config: ClientConfig) -> RunResult {
        use crate::EventBusManager;
        use crate::event_wrapper::EventWrapper;
        use gromnie_client::client::Client;
        use gromnie_events::{EventEnvelope, EventSource, SystemEvent};

        // Create event bus
        let event_bus_manager = Arc::new(EventBusManager::new(self.event_bus_capacity));

        // Setup SIGUSR2 handler if scripting is enabled
        if let Some(ref app_config) = self.app_config
            && app_config.scripting.enabled
        {
            let event_sender = event_bus_manager.create_sender(config.id);
            let script_dir = app_config.scripting.script_dir();
            gromnie_scripting_host::setup_reload_signal_handler(move || {
                let reload_event = EventEnvelope::system_event(
                    SystemEvent::ReloadScripts {
                        script_dir: script_dir.clone(),
                    },
                    0, // client_id (use 0 for system-wide events)
                    0, // sequence
                    EventSource::System,
                );
                event_sender.publish(reload_event);
            });
        }

        // Create raw event channel
        let (raw_event_tx, raw_event_rx) = mpsc::channel::<gromnie_events::ClientEvent>(256);

        // Spawn EventWrapper to bridge client events to event bus
        let event_wrapper = EventWrapper::new(config.id, event_bus_manager.event_bus.clone());
        tokio::spawn(async move {
            event_wrapper.run(raw_event_rx).await;
        });

        // Subscribe to the event bus
        let event_rx = event_bus_manager.subscribe();

        // Create the client
        let (client, action_tx) = Client::new(
            config.id,
            config.address.clone(),
            config.account_name.clone(),
            config.password.clone(),
            raw_event_tx,
        )
        .await;

        // Send action_tx back if requested (for TUI)
        if let Some(ref sender) = self.action_channel {
            let _ = sender.send(action_tx.clone());
        }

        // Create consumer context with default () config type.
        // We use () instead of &ClientConfig to avoid circular dependencies between
        // gromnie-events and gromnie-client. Consumers only need client_id and action_tx.
        let ctx = ConsumerContext {
            client_id: config.id,
            client_config: &(),
            action_tx: action_tx.clone(),
        };

        // Create all consumers
        let consumers: Vec<Box<dyn EventConsumer>> = self
            .consumers
            .iter()
            .map(|factory| factory.create(&ctx))
            .collect();

        // Combine into composite consumer
        let event_consumer = if consumers.len() == 1 {
            consumers.into_iter().next().unwrap()
        } else {
            Box::new(crate::event_consumer::CompositeConsumer::new(consumers))
        };

        // Run the client
        crate::client_runner::run_client_internal(
            client,
            event_rx,
            event_consumer,
            self.shutdown_rx,
        )
        .await;

        RunResult::Single
    }

    /// Run multiple clients with dynamic generation (internal)
    async fn run_dynamic(
        self,
        num_clients: u32,
        server_address: String,
        spawn_interval_ms: u64,
        shared_event_bus: bool,
        generator: Box<dyn Fn(u32) -> ClientConfig + Send + Sync>,
    ) -> RunResult {
        use crate::client_runner::{MultiClientConfig, MultiClientConsumerFactory};

        let multi_config = MultiClientConfig {
            server_address,
            num_clients,
            spawn_interval_ms,
            shared_event_bus,
        };

        // Create a factory adapter
        struct FactoryAdapter {
            consumers: Arc<Vec<Box<dyn ConsumerFactory>>>,
        }

        impl MultiClientConsumerFactory for FactoryAdapter {
            fn create_consumer(
                &self,
                client_id: u32,
                _client_config: &ClientConfig,
                action_tx: mpsc::UnboundedSender<SimpleClientAction>,
            ) -> Box<dyn EventConsumer> {
                // Use () as config type to avoid circular dependencies
                let ctx = ConsumerContext {
                    client_id,
                    client_config: &(),
                    action_tx: action_tx.clone(),
                };

                let consumers: Vec<Box<dyn EventConsumer>> = self
                    .consumers
                    .iter()
                    .map(|factory| factory.create(&ctx))
                    .collect();

                if consumers.len() == 1 {
                    consumers.into_iter().next().unwrap()
                } else {
                    Box::new(crate::event_consumer::CompositeConsumer::new(consumers))
                }
            }
        }

        let factory = Arc::new(FactoryAdapter {
            consumers: Arc::new(self.consumers),
        });

        let stats = crate::client_runner::run_multi_client(
            multi_config,
            factory,
            generator,
            self.shutdown_rx,
        )
        .await;

        RunResult::Multi(stats)
    }

    /// Run multiple clients with static configs (internal)
    async fn run_static(
        self,
        configs: Vec<ClientConfig>,
        spawn_interval_ms: u64,
        shared_event_bus: bool,
    ) -> RunResult {
        let num_clients = configs.len() as u32;
        let server_address = configs
            .first()
            .map(|c| c.address.clone())
            .unwrap_or_else(|| "localhost:9000".to_string());

        // Create generator from static configs
        let configs = Arc::new(configs);
        let generator = move |id: u32| {
            configs.get(id as usize).cloned().unwrap_or_else(|| {
                ClientConfig::new(
                    id,
                    "localhost:9000".into(),
                    format!("client_{}", id),
                    format!("pass_{}", id),
                )
            })
        };

        self.run_dynamic(
            num_clients,
            server_address,
            spawn_interval_ms,
            shared_event_bus,
            Box::new(generator),
        )
        .await
    }
}
