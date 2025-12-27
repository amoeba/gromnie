use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use acprotocol::enums::{Gender, HeritageGroup};
use acprotocol::types::PackableList;
use gromnie::client::events::{ClientAction, GameEvent};
use gromnie::client::{
    ace_protocol::{AceCharGenResult, RawSkillAdvancementClass},
    Client, PendingOutgoingMessage,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    /// connect
    ///
    /// Connect to a server.
    ///
    /// Usage: gromnie connect -a localhost:9000 -u admin -p password
    Connect {
        /// Address to connect to in host:port syntax
        #[arg(short, long, value_name = "ADDRESS")]
        address: Option<String>,

        /// Account name
        #[arg(short, long, value_name = "USERNME")]
        username: Option<String>,

        /// Password
        #[arg(short, long, value_name = "PASSWORD")]
        password: Option<String>,
    },
}

async fn client_task(id: u32, address: String, account_name: String, password: String) {
    let (mut client, mut event_rx, action_tx) = Client::new(
        id.to_owned(),
        address.to_owned(),
        account_name.to_owned(),
        password.to_owned(),
    )
    .await;

    // Track if we've created a character
    let character_created = Arc::new(AtomicBool::new(false));
    let character_created_clone = character_created.clone();

    // Spawn event handler task with action_tx for sending actions back
    tokio::spawn(async move {
        info!(target: "events", "Event handler task started");

        while let Ok(event) = event_rx.recv().await {
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
                    if characters.is_empty() && !character_created_clone.load(Ordering::SeqCst) {
                        info!(target: "events", "No characters found - creating a new character...");

                        // Mark that we're creating a character
                        character_created_clone.store(true, Ordering::SeqCst);

                        // Create a simple character with default values
                        let char_name =
                            format!("TestChar{}", chrono::Utc::now().timestamp() % 10000);

                        // Use ACE-compatible character generation format
                        // ACE expects Heritage and Gender as u32 (not u8), and no redundant account field
                        let char_gen_result = AceCharGenResult::from_generic(
                            HeritageGroup::Aluvian,
                            Gender::Male,
                            0,  // eyes_strip
                            0,  // nose_strip
                            0,  // mouth_strip
                            0,  // hair_color
                            0,  // eye_color
                            0,  // hair_style
                            0,  // headgear_style
                            0,  // headgear_color
                            0,  // shirt_style
                            0,  // shirt_color
                            0,  // trousers_style
                            0,  // trousers_color
                            0,  // footwear_style
                            0,  // footwear_color
                            0,  // skin_shade
                            0,  // hair_shade
                            0,  // headgear_shade
                            0,  // shirt_shade
                            0,  // trousers_shade
                            0,  // tootwear_shade
                            0,  // template_num
                            10, // strength
                            10, // endurance
                            10, // coordination
                            10, // quickness
                            10, // focus
                            10, // self_
                            0,  // slot
                            0,  // class_id
                            {
                                // Create a list of 55 skill entries, all set to Inactive (0)
                                // The server expects exactly 55 skills in SkillAdvancementClass format
                                // ACE defines Inactive = 0, but acprotocol doesn't have it, so we use RawSkillAdvancementClass
                                let mut skills = vec![];
                                for _ in 0..55 {
                                    skills.push(RawSkillAdvancementClass(0));
                                }
                                PackableList {
                                    count: 55,
                                    list: skills,
                                }
                            },
                            char_name.clone(),
                            0, // start_area
                            0, // is_admin
                            0, // is_envoy
                            0, // validation
                        );

                        info!(target: "events", "Creating character: {}", char_name);

                        let msg = PendingOutgoingMessage::CharacterCreationAce(
                            account.clone(),
                            char_gen_result,
                        );
                        if let Err(e) = action_tx.send(ClientAction::SendMessage(msg)) {
                            error!(target: "events", "Failed to send character creation action: {}", e);
                        } else {
                            info!(target: "events", "Character creation action sent - waiting for response...");
                        }
                    }
                    // If we have a character, log in as the first one
                    else if !characters.is_empty() {
                        info!(target: "events", "Found existing character(s):");
                        for char_info in &characters {
                            info!(target: "events", "  Character: {} (ID: {})", char_info.name, char_info.id);
                        }

                        // Log in as the first character
                        let first_char = &characters[0];
                        info!(target: "events", "Attempting to log in as: {} (ID: {})", first_char.name, first_char.id);

                        // Send action to login
                        if let Err(e) = action_tx.send(ClientAction::LoginCharacter {
                            character_id: first_char.id,
                            character_name: first_char.name.clone(),
                            account: account.clone(),
                        }) {
                            error!(target: "events", "Failed to send login action: {}", e);
                        } else {
                            // Store the character for later use
                            character_created_clone.store(true, Ordering::SeqCst);
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
                    // LoginComplete is already sent by the client when 0xF746 is received
                    
                    // Send a chat message after successful login
                    info!(target: "events", "Sending chat message...");
                    if let Err(e) = action_tx.send(ClientAction::SendChatMessage {
                        message: "Hello from gromnie!".to_string(),
                    }) {
                        error!(target: "events", "Failed to send chat message: {}", e);
                    }
                }
                GameEvent::LoginFailed { reason } => {
                    error!(target: "events", "=== LOGIN FAILED ===");
                    error!(target: "events", "Reason: {}", reason);
                }
                GameEvent::CreateObject {
                    object_id,
                    object_name,
                } => {
                    info!(target: "events", "CREATE OBJECT: {} (0x{:08X})", object_name, object_id);
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
            }
        }

        info!(target: "events", "Event handler task stopped");
    });

    // TODO: Handle propertly
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
    let keepalive_interval = tokio::time::Duration::from_secs(10); // Send keep-alive every 10 seconds

    loop {
        // Use tokio::select! to either receive a packet, timeout for keep-alive, or handle Ctrl+C
        tokio::select! {
            recv_result = client.socket.recv_from(&mut buf) => {
                match recv_result {
                    Ok((size, peer)) => {
                        client.process_packet(&buf[..size], size, &peer).await;

                        // Check for and process any messages that were parsed from fragments
                        if client.has_messages() {
                            client.process_messages();
                        }

                        // Process actions from event handlers
                        client.process_actions();

                        // Check for and send any pending outgoing messages
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
                // Time to send keep-alive
                if let Err(e) = client.send_keepalive().await {
                    error!("Failed to send keep-alive: {}", e);
                }
                last_keepalive = tokio::time::Instant::now();
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down gracefully...");
                break;
            }
        }
    }

    info!("Client task shutting down");
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Initialize tracing subscriber with env filter
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // TODO: Finish CLI
    let _ = Cli::parse();

    // TODO: Wrap this up nicer
    let address = "localhost:9000";
    let account_name_prefix = "user";
    let _password = "password";

    let n = 1;
    let mut tasks = Vec::with_capacity(2);

    for i in 0..n {
        let mut account_name = account_name_prefix.to_owned();
        let suffix = i.to_string();
        account_name.push_str(suffix.as_ref());

        tasks.push(tokio::spawn(client_task(
            i.to_owned(),
            address.to_owned(),
            "testing".to_owned(),
            "testing".to_owned(),
        )));
    }

    // Wait for either all tasks to complete or Ctrl+C
    tokio::select! {
        _ = async {
            for task in tasks {
                let _ = task.await;
            }
        } => {
            info!("All client tasks completed");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C in main, shutting down...");
        }
    }

    Ok(())
}
