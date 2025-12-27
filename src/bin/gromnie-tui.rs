use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use acprotocol::enums::{Gender, HeritageGroup};
use acprotocol::types::PackableList;
use gromnie::client::events::{ClientAction, GameEvent};
use gromnie::client::{
    ace_protocol::{AceCharGenConfig, AceCharGenResult, RawSkillAdvancementClass},
    Client, OutgoingMessageContent,
};
use gromnie::tui::{event_handler::EventHandler, ui::try_init_tui, App};
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Address to connect to in host:port syntax
    #[arg(short, long, value_name = "ADDRESS", default_value = "localhost:9000")]
    address: String,

    /// Account name
    #[arg(short, long, value_name = "USERNAME", default_value = "testing")]
    username: String,

    /// Password
    #[arg(short, long, value_name = "PASSWORD", default_value = "testing")]
    password: String,
}

async fn client_task(
    id: u32,
    address: String,
    account_name: String,
    password: String,
    event_tx: tokio::sync::mpsc::UnboundedSender<GameEvent>,
    action_tx_sender: mpsc::UnboundedSender<mpsc::UnboundedSender<ClientAction>>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    let (mut client, mut event_rx, action_tx) = Client::new(
        id.to_owned(),
        address.to_owned(),
        account_name.to_owned(),
        password.to_owned(),
    )
    .await;

    // Send the action_tx channel back to the TUI so it can send login actions
    let _ = action_tx_sender.send(action_tx.clone());

    // Track if we've created a character
    let character_created = Arc::new(AtomicBool::new(false));
    let character_created_clone = character_created.clone();

    // Spawn event handler task with action_tx for sending actions back
    tokio::spawn({
        let tx = event_tx.clone();
        async move {
            info!(target: "events", "Event handler task started");

            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Forward to TUI
                        let _ = tx.send(event.clone());

                        match event {
                            GameEvent::CharacterListReceived {
                                account,
                                characters,
                                num_slots,
                            } => {
                                info!(target: "events", "=== Character List Event ===");
                                info!(target: "events", "Account: {}", account);
                                info!(target: "events", "Slots: {}", num_slots);
                                info!(target: "events", "Number of characters: {}", characters.len());

                                // Print character names
                                for char_info in &characters {
                                    if char_info.delete_pending {
                                        info!(target: "events", "  - {} (ID: {}) [PENDING DELETION]", char_info.name, char_info.id);
                                    } else {
                                        info!(target: "events", "  - {} (ID: {})", char_info.name, char_info.id);
                                    }
                                }

                                // If we don't have any characters, create one
                                if characters.is_empty()
                                    && !character_created_clone.load(Ordering::SeqCst)
                                {
                                    info!(target: "events", "No characters found - creating a new character...");

                                    // Mark that we're creating a character
                                    character_created_clone.store(true, Ordering::SeqCst);

                                    // Create a simple character with default values
                                    let char_name = format!(
                                        "TestChar{}",
                                        chrono::Utc::now().timestamp() % 10000
                                    );

                                    // Use ACE-compatible character generation format
                                    let char_gen_result =
                                        AceCharGenResult::from_generic(AceCharGenConfig {
                                            heritage: HeritageGroup::Aluvian,
                                            gender: Gender::Male,
                                            eyes_strip: 0,     // eyes_strip
                                            nose_strip: 0,     // nose_strip
                                            mouth_strip: 0,    // mouth_strip
                                            hair_color: 0,     // hair_color
                                            eye_color: 0,      // eye_color
                                            hair_style: 0,     // hair_style
                                            headgear_style: 0, // headgear_style
                                            headgear_color: 0, // headgear_color
                                            shirt_style: 0,    // shirt_style
                                            shirt_color: 0,    // shirt_color
                                            trousers_style: 0, // trousers_style
                                            trousers_color: 0, // trousers_color
                                            footwear_style: 0, // footwear_style
                                            footwear_color: 0, // footwear_color
                                            skin_shade: 0,     // skin_shade
                                            hair_shade: 0,     // hair_shade
                                            headgear_shade: 0, // headgear_shade
                                            shirt_shade: 0,    // shirt_shade
                                            trousers_shade: 0, // trousers_shade
                                            tootwear_shade: 0, // tootwear_shade
                                            template_num: 0,   // template_num
                                            strength: 10,      // strength
                                            endurance: 10,     // endurance
                                            coordination: 10,  // coordination
                                            quickness: 10,     // quickness
                                            focus: 10,         // focus
                                            self_: 10,         // self_
                                            slot: 0,           // slot
                                            class_id: 0,       // class_id
                                            skills: {
                                                // Create a list of 55 skill entries, all set to Inactive (0)
                                                let mut skills = vec![];
                                                for _ in 0..55 {
                                                    skills.push(RawSkillAdvancementClass(0));
                                                }
                                                PackableList {
                                                    count: 55,
                                                    list: skills,
                                                }
                                            },
                                            name: char_name.clone(),
                                            start_area: 0, // start_area
                                            is_admin: 0,   // is_admin
                                            is_envoy: 0,   // is_envoy
                                            validation: 0, // validation
                                        });

                                    info!(target: "events", "Creating character: {}", char_name);

                                    let msg = OutgoingMessageContent::CharacterCreationAce(
                                        account.clone(),
                                        char_gen_result,
                                    );
                                    if let Err(e) =
                                        action_tx.send(ClientAction::SendMessage(Box::new(msg)))
                                    {
                                        error!(target: "events", "Failed to send character creation action: {}", e);
                                    } else {
                                        info!(target: "events", "Character creation action sent - waiting for response...");
                                    }
                                }
                                // Character list received - user will select from TUI
                                else if !characters.is_empty() {
                                    info!(target: "events", "Found existing character(s):");
                                    for char_info in &characters {
                                        info!(target: "events", "  Character: {} (ID: {})", char_info.name, char_info.id);
                                    }
                                }
                            }
                            GameEvent::DDDInterrogation { language, region } => {
                                info!(target: "events", "DDD Interrogation: lang={} region={}", language, region);
                            }
                            GameEvent::LoginSucceeded {
                                character_id,
                                character_name,
                            } => {
                                info!(target: "events", "=== LOGIN SUCCEEDED === Character: {} (ID: {}) | You are now in the game world!", character_name, character_id);
                            }
                            GameEvent::LoginFailed { reason } => {
                                error!(target: "events", "=== LOGIN FAILED ===");
                                error!(target: "events", "Reason: {}", reason);
                            }
                            GameEvent::CreateObject {
                                object_id,
                                object_name,
                            } => {
                                debug!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
                            }
                            GameEvent::ChatMessageReceived {
                                message,
                                message_type,
                            } => {
                                info!(target: "events", "CHAT [{}]: {}", message_type, message);
                            }
                            GameEvent::NetworkMessage {
                                direction,
                                message_type,
                            } => {
                                debug!(target: "events", "Network message: {:?} - {}", direction, message_type);
                            }
                            GameEvent::ConnectingSetProgress { progress: _ } => {
                                // These events are handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::UpdatingSetProgress { progress: _ } => {
                                // These events are handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::FakeProgressComplete => {
                                // This event is handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::ConnectingStart => {
                                // This event is handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::ConnectingDone => {
                                // This event is handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::UpdatingStart => {
                                // This event is handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                            GameEvent::UpdatingDone => {
                                // This event is handled in the TUI app directly
                                // via the scheduled events mechanism
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        // Receiver lagged - log and continue
                        error!(target: "events", "Event receiver lagged, {} messages were skipped", skipped);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Channel closed - exit task
                        info!(target: "events", "Event channel closed");
                        break;
                    }
                }
            }

            info!(target: "events", "Event handler task stopped");
        }
    });

    // TODO: Handle properly
    match client.connect().await {
        Ok(_) => {}
        Err(e) => {
            error!("Connect failed: {}", e);
            panic!("Connect failed");
        }
    };

    // TODO: Handle properly
    match client.do_login().await {
        Ok(_) => {}
        Err(e) => {
            error!("Login failed: {}", e);
            panic!("Login failed");
        }
    }

    let mut buf = [0u8; 1024];
    let mut last_keepalive = tokio::time::Instant::now();
    let keepalive_interval = tokio::time::Duration::from_secs(10);

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

                        if client.has_pending_outgoing_messages() {
                            if let Err(e) = client.send_pending_messages().await {
                                error!("Failed to send pending messages: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error in receive loop: {}", e);
                        break;
                    }
                }
            }
            _ = tokio::time::sleep_until(last_keepalive + keepalive_interval) => {
                if let Err(e) = client.send_keepalive().await {
                    error!("Failed to send keep-alive: {}", e);
                }
                last_keepalive = tokio::time::Instant::now();
            }
            _ = shutdown_rx.changed() => {
                info!("Client task received shutdown signal");
                break;
            }
        }
    }

    info!("Client task shutting down - cleaning up network connections...");
    // Socket will be closed when client is dropped
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber with all output disabled to prevent TUI corruption
    // Set RUST_LOG=info to see error messages
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off")),
        )
        .init();

    let cli = Cli::parse();

    // Initialize TUI
    let mut tui = try_init_tui()?;
    let mut app = App::new();

    // Mark client as connected when we start
    app.client_status.connected = true;

    // Set up event handler
    let (event_handler, mut tui_event_rx) = EventHandler::new();
    let event_handler = event_handler.start().await;

    // Spawn the client task - it will return a receiver we can use
    // For now, we'll spawn it and communicate through a channel we create
    let (client_event_tx, mut client_event_rx) = tokio::sync::mpsc::unbounded_channel();
    let (action_tx_channel, mut action_tx_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create shutdown channel to coordinate graceful shutdown
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut client_handle = tokio::spawn(client_task(
        0,
        cli.address,
        cli.username,
        cli.password,
        client_event_tx,
        action_tx_channel,
        shutdown_rx,
    ));

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
        // Draw current UI (only if enough time has passed and state has changed)
        let now = std::time::Instant::now();
        if now.duration_since(last_render_time) >= min_render_interval {
            tui.draw(&app)?;
            last_render_time = now;
        }

        // Process any scheduled events that are due
        app.process_scheduled_events();

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
            Some(game_event) = client_event_rx.recv() => {
                // Handle game events through centralized message passing
                app.update_from_event(game_event);
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
    let timeout = tokio::time::Duration::from_secs(2);
    match tokio::time::timeout(timeout, client_handle).await {
        Ok(result) => match result {
            Ok(_) => info!("Client task shut down gracefully"),
            Err(e) => error!("Client task panicked: {}", e),
        },
        Err(_) => {
            error!("Client task did not shut down within timeout, proceeding anyway");
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
    tui_event: gromnie::tui::event_handler::TuiEvent,
    shutdown_tx: &tokio::sync::watch::Sender<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use gromnie::tui::event_handler::TuiEvent;
    use tracing::{error, info};

    match tui_event {
        TuiEvent::Key(key) => {
            // ALWAYS handle Tab/BackTab for GameWorld tab switching
            if matches!(
                app.game_scene,
                gromnie::tui::app::GameScene::GameWorld { .. }
            ) && matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
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
            } else if app.chat_input_active {
                match key.code {
                    KeyCode::Enter => {
                        // Send chat message if there's text
                        if !app.chat_input.is_empty() {
                            if let Some(ref tx) = app.action_tx {
                                let message = app.chat_input.clone();
                                if let Err(e) = tx.send(ClientAction::SendChatMessage { message }) {
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
                if app.current_view == gromnie::tui::app::AppView::Game {
                    match key.code {
                        KeyCode::Up => {
                            app.select_previous_character();
                        }
                        KeyCode::Down => {
                            app.select_next_character();
                        }
                        KeyCode::Enter => {
                            // If in GameWorld scene and on Chat tab, activate chat input
                            if matches!(
                                app.game_scene,
                                gromnie::tui::app::GameScene::GameWorld { .. }
                            ) && app.game_world_tab == gromnie::tui::app::GameWorldTab::Chat
                            {
                                // Activate chat input when on Chat tab
                                app.chat_input_active = true;
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
                            // Activate chat input when in game world
                            if matches!(
                                app.game_scene,
                                gromnie::tui::app::GameScene::GameWorld { .. }
                            ) {
                                app.chat_input_active = true;
                            }
                        }
                        _ => {}
                    }
                }

                // Handle global controls (only when chat input is not active)
                if !app.chat_input_active {
                    match key.code {
                        KeyCode::Char('1') => {
                            app.switch_view(gromnie::tui::app::AppView::Game);
                        }
                        KeyCode::Char('2') => {
                            app.switch_view(gromnie::tui::app::AppView::Debug);
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
