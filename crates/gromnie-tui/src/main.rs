use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use gromnie_client::config::GromnieConfig;
use gromnie_events::SimpleClientAction;
use gromnie_runner::{ClientConfig, ClientRunner, TuiConsumer, TuiEvent};
use gromnie_tui::{App, event_handler::EventHandler, ui::try_init_tui};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Server to connect to (name from config)
    #[arg(short, long)]
    server: Option<String>,

    /// Account to use (name from config)
    #[arg(short, long)]
    account: Option<String>,

    /// Server host
    #[arg(long)]
    host: Option<String>,

    /// Server port
    #[arg(long)]
    port: Option<u16>,

    /// Account password
    #[arg(long)]
    password: Option<String>,

    /// Character name to auto-login
    #[arg(long)]
    character: Option<String>,

    /// Enable automatic reconnection on connection loss
    #[arg(long)]
    reconnect: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing subscriber with file logging if GROMNIE_DEBUG=1
    if std::env::var("GROMNIE_DEBUG").is_ok() {
        let log_file = std::fs::File::create("gromnie_debug.log")?;
        let env_filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::level_filters::LevelFilter::DEBUG.into());
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::sync::Mutex::new(log_file))
            .init();
        info!("Debug logging enabled: writing to gromnie_debug.log");
    } else {
        // Minimal logging to stderr when not in debug mode
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    // Load configuration if available, but only require it when using --server/--account aliases
    let config = match GromnieConfig::load() {
        Ok(cfg) => {
            info!(
                "Loaded config from {}",
                GromnieConfig::config_path().display()
            );
            cfg
        }
        Err(_) => {
            // Only error if using config-based connection (--server/--account)
            if cli.host.is_none() && cli.port.is_none() && cli.password.is_none() {
                eprintln!(
                    "Config file not found at {}. Please create it with servers and accounts.",
                    GromnieConfig::config_path().display()
                );
                eprintln!("Example config:");
                eprintln!("[servers.local]");
                eprintln!("host = \"localhost\"");
                eprintln!("port = 9000");
                eprintln!();
                eprintln!("[accounts.testing]");
                eprintln!("username = \"testing\"");
                eprintln!("password = \"testing\"");
                return Err("Config file not found".into());
            }
            // Use empty config when using direct CLI connection
            GromnieConfig::default()
        }
    };

    // Determine connection parameters: CLI args take precedence, then fall back to config
    let (host, port, account_name, password, character_name) =
        if cli.host.is_some() || cli.port.is_some() || cli.password.is_some() {
            // Using CLI arguments for connection
            let host = cli
                .host
                .ok_or("--host is required when using direct connection")?;
            let port = cli
                .port
                .ok_or("--port is required when using direct connection")?;
            let account_name = cli
                .account
                .ok_or("--account is required when using direct connection")?;
            let password = cli
                .password
                .ok_or("--password is required when using direct connection")?;
            let character_name = cli.character;

            info!(
                "Connecting to server {}:{} with account '{}' (from CLI args)",
                host, port, account_name
            );
            (host, port, account_name, password, character_name)
        } else {
            // Using config file
            let server_name = cli
                .server
                .ok_or("Either --server (config name) or --host/--port must be specified")?;
            let account_name = cli.account.ok_or(
                "Either --account (config name) or --account/--password must be specified",
            )?;

            let server = config.servers.get(&server_name).ok_or_else(|| {
                let available = config
                    .servers
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Server '{}' not found. Available: {}",
                    server_name, available
                )
            })?;

            let account = config.accounts.get(&account_name).ok_or_else(|| {
                let available = config
                    .accounts
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Account '{}' not found. Available: {}",
                    account_name, available
                )
            })?;

            info!(
                "Connecting to server '{}' with account '{}' (from config)",
                server_name, account_name
            );

            (
                server.host.clone(),
                server.port,
                account.username.clone(),
                account.password.clone(),
                None, // Don't auto-login character
            )
        };

    let address = format!("{}:{}", host, port);

    // Initialize TUI
    let mut tui = try_init_tui()?;
    let mut app = App::new();

    // Set up event handler
    let (event_handler, mut tui_event_rx) = EventHandler::new();
    let event_handler = event_handler.start().await;

    // Set up channels for client communication
    let (client_event_tx, mut client_event_rx) = tokio::sync::mpsc::unbounded_channel::<TuiEvent>();
    let (action_tx_channel, mut action_tx_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create shutdown channel to coordinate graceful shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Create client configuration
    let client_config = ClientConfig {
        id: 0,
        address,
        account_name,
        password,
        // CLI flag overrides config file
        reconnect: cli.reconnect || config.reconnect,
        character_name,
    };

    // Spawn client task using the runner module
    let runner = ClientRunner::builder()
        .with_clients(client_config)
        .with_consumer(TuiConsumer::from_factory(client_event_tx))
        .with_action_channel(action_tx_channel)
        .with_shutdown(shutdown_rx)
        .with_config(config)
        .build()
        .expect("Failed to build client runner");

    let mut client_handle = tokio::spawn(async move {
        runner.run().await;
    });

    // Wait for the action_tx channel from the client task (with timeout)
    match tokio::time::timeout(tokio::time::Duration::from_secs(5), action_tx_rx.recv()).await {
        Ok(Some(action_tx)) => {
            app.action_tx = Some(action_tx);
        }
        _ => {
            error!("Failed to receive action_tx from client task");
        }
    }

    // Main TUI loop
    let mut last_render_time = std::time::Instant::now();
    let min_render_interval = std::time::Duration::from_millis(16); // ~60 FPS max

    loop {
        // Draw current UI (only if enough time has passed)
        let now = std::time::Instant::now();
        if now.duration_since(last_render_time) >= min_render_interval {
            tui.draw(&app)?;
            last_render_time = now;
        }

        // Centralized event polling and handling
        tokio::select! {
            Some(tui_event) = tui_event_rx.recv() => {
                // Handle TUI events through centralized message passing
                if let Err(e) = handle_tui_event(&mut app, tui_event, &shutdown_tx) {
                    error!("Error handling TUI event: {}", e);
                    break;
                }

                // Check if the event handler requested to quit
                if app.should_quit {
                    break;
                }
            }
            Some(tui_event) = client_event_rx.recv() => {
                // Handle game events through centralized message passing
                match tui_event {
                    TuiEvent::Game(game_event) => {
                        tracing::info!(target: "tui_main", "TUI main received GameEvent: {:?}", std::mem::discriminant(&game_event));
                        app.update_from_event(game_event);
                    }
                    TuiEvent::System(system_event) => {
                        tracing::info!(target: "tui_main", "TUI main received SystemEvent: {:?}", std::mem::discriminant(&system_event));
                        app.update_from_system_event(system_event);
                    }
                    TuiEvent::State(state_event) => {
                        tracing::info!(target: "tui_main", "TUI main received StateEvent: {:?}", std::mem::discriminant(&state_event));
                        app.update_from_state_event(state_event);
                    }
                }
            }
            // Check if client task exited
            _ = &mut client_handle => {
                info!("Client task finished");
                break;
            }
        }

        if app.should_quit {
            // Signal client task to shut down
            let _ = shutdown_tx.send(true);
            break;
        }
    }

    info!("TUI shutting down - waiting for client task to finish...");

    // Give client task a moment to clean up gracefully
    let timeout = tokio::time::Duration::from_millis(250);
    match tokio::time::timeout(timeout, client_handle).await {
        Ok(result) => match result {
            Ok(_) => info!("Client task shut down gracefully"),
            Err(e) => error!("Client task panicked: {}", e),
        },
        Err(_) => {
            info!("Client task did not shut down within timeout, proceeding anyway");
        }
    }

    info!("TUI shut down cleanly");

    // Explicitly restore terminal before exiting
    drop(tui);

    // Shutdown event handler task
    event_handler.shutdown();

    Ok(())
}

// Handle TUI events in a centralized function
fn handle_tui_event(
    app: &mut App,
    tui_event: gromnie_tui::event_handler::TuiEvent,
    shutdown_tx: &tokio::sync::watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use gromnie_tui::event_handler::TuiEvent;
    use tracing::{error, info};

    match tui_event {
        TuiEvent::Key(key) => {
            // Handle Tab/BackTab for GameWorld tab switching (only when not in portal space)
            if matches!(
                app.game_scene,
                gromnie_tui::app::GameScene::GameWorld { .. }
            ) && matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
            {
                // Prevent tab switching while in portal space
                if let gromnie_tui::app::GameScene::GameWorld { ref state, .. } = app.game_scene
                    && *state != gromnie_tui::app::GameWorldState::InPortalSpace
                {
                    match key.code {
                        KeyCode::Tab => {
                            app.next_tab();
                        }
                        KeyCode::BackTab => {
                            app.previous_tab();
                        }
                        _ => unreachable!(),
                    }
                }
            } else if app.chat_input_active {
                match key.code {
                    KeyCode::Enter => {
                        // Send chat message if there's text
                        if !app.chat_input.is_empty() {
                            if let Some(ref tx) = app.action_tx {
                                let message = app.chat_input.clone();
                                if let Err(e) = tx.send(SimpleClientAction::SendChatSay { message })
                                {
                                    error!("Failed to send chat message action: {}", e);
                                }
                            }
                            // Clear input
                            app.chat_input.clear();
                        }
                        // Deactivate chat input after sending
                        app.chat_input_active = false;
                    }
                    KeyCode::Esc => {
                        // Cancel chat input
                        app.chat_input.clear();
                        app.chat_input_active = false;
                    }
                    KeyCode::Backspace => {
                        // Delete last character
                        app.chat_input.pop();
                    }
                    KeyCode::Char(c) => {
                        // Add character to input (but don't process control keys like Ctrl+C)
                        if !key.modifiers.contains(KeyModifiers::CONTROL) {
                            app.chat_input.push(c);
                        }
                    }
                    _ => {}
                }
            } else {
                // Handle GameView character selection controls
                if app.current_view == gromnie_tui::app::AppView::Game {
                    match key.code {
                        KeyCode::Up => {
                            app.select_previous_character();
                        }
                        KeyCode::Down => {
                            app.select_next_character();
                        }
                        KeyCode::Enter => {
                            // If in GameWorld scene and on Chat tab, activate chat input (but not in portal space)
                            if let gromnie_tui::app::GameScene::GameWorld { ref state, .. } =
                                app.game_scene
                            {
                                if app.game_world_tab == gromnie_tui::app::GameWorldTab::Chat {
                                    // Only allow chat input when not in portal space
                                    if *state != gromnie_tui::app::GameWorldState::InPortalSpace {
                                        app.chat_input_active = true;
                                    }
                                } else {
                                    // Try to login with selected character (in other scenes)
                                    match app.login_selected_character() {
                                        Ok(_) => {
                                            info!("Logging in with selected character");
                                        }
                                        Err(e) => {
                                            error!("Failed to login: {}", e);
                                        }
                                    }
                                }
                            } else {
                                // Try to login with selected character (in other scenes)
                                match app.login_selected_character() {
                                    Ok(_) => {
                                        info!("Logging in with selected character");
                                    }
                                    Err(e) => {
                                        error!("Failed to login: {}", e);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            // Activate chat input when in game world (but not during portal space)
                            if let gromnie_tui::app::GameScene::GameWorld { ref state, .. } =
                                app.game_scene
                            {
                                // Only allow chat input when not in portal space
                                if *state != gromnie_tui::app::GameWorldState::InPortalSpace {
                                    app.chat_input_active = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Handle global controls (only when chat input is not active)
                if !app.chat_input_active {
                    match key.code {
                        KeyCode::Char('1') => {
                            app.switch_view(gromnie_tui::app::AppView::Game);
                        }
                        KeyCode::Char('2') => {
                            app.switch_view(gromnie_tui::app::AppView::Debug);
                        }
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                            return Ok(()); // Return Ok to break from main loop
                        }
                        _ => {}
                    }
                }
            }
        }
        TuiEvent::Quit => {
            info!("Received Ctrl+C signal, initiating graceful shutdown...");
            app.should_quit = true;
            // Signal client task to shut down
            let _ = shutdown_tx.send(true);
        }
        TuiEvent::Tick => {
            // Periodic update opportunity - currently unused
        }
    }
    Ok(())
}
