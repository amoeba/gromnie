use clap::Parser;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use gromnie::client::events::{ClientAction, GameEvent};
use gromnie::client::OutgoingMessageContent;
use gromnie::runner::{ClientConfig, EventConsumer, CharacterBuilder};

mod naming;
use naming::ClientNaming;

#[derive(Parser)]
#[command(name = "load-tester")]
#[command(about = "Load testing tool for AC server")]
pub struct Args {
    /// Number of clients to spawn
    #[arg(short, long, default_value = "5")]
    clients: u32,

    /// Server host address
    #[arg(long, default_value = "localhost")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "9000")]
    port: u16,

    /// Delay between client connections in seconds
    #[arg(short, long, default_value = "1")]
    rate_limit: u64,

    /// Enable verbose per-client logging
    #[arg(short, long)]
    verbose: bool,

    /// Stats display interval in seconds
    #[arg(long, default_value = "5")]
    stats_interval: u64,
}

/// Statistics collector for all clients
#[derive(Default)]
struct EventCounts {
    authenticated: AtomicU32,
    character_created: AtomicU32,
    logged_in: AtomicU32,
    errors: AtomicU32,
}

/// Load tester client state machine
#[derive(Clone, Debug, PartialEq)]
enum LoadTesterState {
    /// Waiting for character list, haven't found our character yet
    WaitingForCharList,
    /// Character not found in list, creation in progress
    CharacterCreationInProgress,
    /// Character found in list, ready to log in
    CharacterFound,
}

/// Event consumer for load testing
struct LoadTesterConsumer {
    client_id: u32,
    character_name: String,
    event_counts: Arc<EventCounts>,
    action_tx: mpsc::UnboundedSender<ClientAction>,
    verbose: bool,
    state: LoadTesterState,
}

impl LoadTesterConsumer {
    fn new(
        client_id: u32,
        character_name: String,
        event_counts: Arc<EventCounts>,
        action_tx: mpsc::UnboundedSender<ClientAction>,
        verbose: bool,
    ) -> Self {
        Self {
            client_id,
            character_name,
            event_counts,
            action_tx,
            verbose,
            state: LoadTesterState::WaitingForCharList,
        }
    }
}

impl EventConsumer for LoadTesterConsumer {
    fn handle_event(&mut self, event: GameEvent) {
        match event {
            GameEvent::AuthenticationSucceeded => {
                self.event_counts.authenticated.fetch_add(1, Ordering::SeqCst);
                if self.verbose {
                    info!("[Client {}] Authentication succeeded", self.client_id);
                }
            }
            GameEvent::LoginSucceeded {
                character_name,
                character_id,
            } => {
                self.event_counts.logged_in.fetch_add(1, Ordering::SeqCst);
                if self.verbose {
                    info!(
                        "[Client {}] Logged in as {} (ID: {})",
                        self.client_id, character_name, character_id
                    );
                }
            }
            GameEvent::AuthenticationFailed { reason } => {
                self.event_counts.errors.fetch_add(1, Ordering::SeqCst);
                error!("[Client {}] Auth failed: {}", self.client_id, reason);
            }
            GameEvent::LoginFailed { reason } => {
                self.event_counts.errors.fetch_add(1, Ordering::SeqCst);
                error!("[Client {}] Login failed: {}", self.client_id, reason);
            }
            GameEvent::CharacterListReceived {
                characters,
                account,
                num_slots: _,
            } => {
                if self.verbose {
                    info!(
                        "[Client {}] Got character list for {}: {} chars",
                        self.client_id,
                        account,
                        characters.len()
                    );
                }

                // Handle based on current state
                match self.state {
                    LoadTesterState::WaitingForCharList | LoadTesterState::CharacterCreationInProgress => {
                        // Check if our character exists
                        if let Some(char_info) = characters.iter().find(|c| c.name == self.character_name) {
                            // Character found (either was there initially or just created)
                            if self.verbose {
                                info!(
                                    "[Client {}] Found character: {} (ID: {})",
                                    self.client_id, char_info.name, char_info.id
                                );
                            }
                            // Update state and proceed to login
                            self.state = LoadTesterState::CharacterFound;
                            if let Err(e) = self.action_tx.send(ClientAction::LoginCharacter {
                                character_id: char_info.id,
                                character_name: char_info.name.clone(),
                                account: account.clone(),
                            }) {
                                error!("[Client {}] Failed to send login action: {}", self.client_id, e);
                            }
                        } else if self.state == LoadTesterState::WaitingForCharList {
                            // Character doesn't exist yet - create it
                            if self.verbose {
                                info!("[Client {}] Creating character: {}", self.client_id, self.character_name);
                            }
                            self.event_counts.character_created.fetch_add(1, Ordering::SeqCst);
                            self.state = LoadTesterState::CharacterCreationInProgress;

                            let char_gen_result = CharacterBuilder::new(self.character_name.clone()).build();
                            let msg = OutgoingMessageContent::CharacterCreationAce(
                                account.clone(),
                                char_gen_result,
                            );
                            if let Err(e) = self.action_tx.send(ClientAction::SendMessage(Box::new(msg))) {
                                error!("[Client {}] Failed to send character creation: {}", self.client_id, e);
                            }
                        }
                    }
                    LoadTesterState::CharacterFound => {
                        // Already found and logging in, ignore further character list updates
                        if self.verbose {
                            info!("[Client {}] Already processing login, ignoring character list update", self.client_id);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    info!("Starting load tester: {} clients to {}:{}", args.clients, args.host, args.port);
    info!("Rate limiting: {} sec between connections", args.rate_limit);

    let event_counts = Arc::new(EventCounts::default());
    let (shutdown_tx, _) = watch::channel(false);
    let mut join_handles = vec![];

    let start_time = Instant::now();

    // Spawn client tasks
    for client_id in 0..args.clients {
        let host = args.host.clone();
        let port = args.port;
        let shutdown_rx = shutdown_tx.subscribe();
        let event_counts = event_counts.clone();
        let verbose = args.verbose;
        let rate_limit = args.rate_limit;

        let handle = tokio::spawn(async move {
            // Rate limiting: stagger client connections
            tokio::time::sleep(Duration::from_secs(client_id as u64 * rate_limit)).await;

            let naming = ClientNaming::new(client_id);
            let account_name = naming.account_name();
            let password = naming.password();
            let character_name = naming.character_name();

            let address = format!("{}:{}", host, port);

            let client_config = ClientConfig {
                id: client_id,
                address,
                account_name,
                password,
            };

            let event_counts = event_counts.clone();
            let consumer_factory = move |action_tx| {
                LoadTesterConsumer::new(
                    client_id,
                    character_name.clone(),
                    event_counts.clone(),
                    action_tx,
                    verbose,
                )
            };

            gromnie::runner::run_client(client_config, consumer_factory, Some(shutdown_rx))
                .await;
        });

        join_handles.push(handle);
    }

    info!("All clients spawned, waiting for events...");

    // Stats display task
    let event_counts_stats = event_counts.clone();
    let stats_interval = args.stats_interval;
    let stats_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(stats_interval)).await;

            let elapsed = start_time.elapsed().as_secs();
            let auth = event_counts_stats.authenticated.load(Ordering::SeqCst);
            let login = event_counts_stats.logged_in.load(Ordering::SeqCst);
            let char_created = event_counts_stats.character_created.load(Ordering::SeqCst);
            let errors = event_counts_stats.errors.load(Ordering::SeqCst);

            info!(
                "[Stats @ {}s] Auth: {} | LoggedIn: {} | CharCreated: {} | Errors: {}",
                elapsed, auth, login, char_created, errors
            );
        }
    });

    // Wait for Ctrl+C
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Received Ctrl+C, shutting down all clients...");
            let _ = shutdown_tx.send(true);
        }
        Err(e) => {
            error!("Failed to listen for Ctrl+C: {}", e);
        }
    }

    // Wait for all client tasks to complete
    for handle in join_handles {
        let _ = handle.await;
    }

    stats_handle.abort();

    // Display final stats
    let total_time = start_time.elapsed();
    let auth = event_counts.authenticated.load(Ordering::SeqCst);
    let login = event_counts.logged_in.load(Ordering::SeqCst);
    let errors = event_counts.errors.load(Ordering::SeqCst);

    info!("Load test complete");
    info!("Total time: {:.2}s", total_time.as_secs_f64());
    info!("Total authenticated: {}", auth);
    info!("Total logged in: {}", login);
    info!("Total errors: {}", errors);
    info!("Success rate: {:.1}%", (login as f64 / args.clients as f64) * 100.0);
}
