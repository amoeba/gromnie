use clap::Parser;
use std::sync::Arc;
use serenity::all::{
    CommandInteraction, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
    Interaction,
};
use serenity::client::Context;
use serenity::model::gateway::Ready;
use serenity::prelude::{EventHandler, GatewayIntents};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use gromnie_client::client::events::{ClientAction, GameEvent};
use gromnie_runner::{ClientConfig, EventBusManager, DiscordConsumer, UptimeData};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

struct Handler {
    target_channel_id: serenity::model::id::ChannelId,
    action_tx: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<ClientAction>>>>,
    uptime_data: Arc<RwLock<UptimeData>>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Discord bot logged in as: {}", ready.user.name);

        // Register slash command
        let uptime_command =
            CreateCommand::new("uptime").description("Show bot uptime and in-game time");

        if let Err(e) = ctx.http.create_global_command(&uptime_command).await {
            error!("Failed to create uptime command: {}", e);
        } else {
            info!("Slash commands registered");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            info!(
                "Received command {} from user {}",
                command.data.name, command.user.id
            );

            match command.data.name.as_str() {
                "uptime" => self.handle_uptime_command(&ctx, &command).await,
                _ => {
                    let data = CreateInteractionResponseMessage::new()
                        .content("Unknown command".to_string());
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(e) = command.create_response(&ctx.http, builder).await {
                        error!("Failed to respond to unknown command: {}", e);
                    }
                }
            }
        }
    }

    async fn message(&self, _: Context, msg: serenity::model::channel::Message) {
        // Ignore bot messages
        if msg.author.bot {
            return;
        }

        // Check if this is a DM (private message, not in a guild)
        if msg.guild_id.is_none() {
            println!("[DM from {}]: {}", msg.author.name, msg.content);
            return;
        }

        // Check if this message is in the target channel
        if msg.channel_id == self.target_channel_id {
            // Forward to game
            let action_tx = self.action_tx.lock().await;
            if let Some(ref tx) = *action_tx {
                let game_message = format!("Discord: {}: {}", msg.author.name, msg.content);
                if let Err(e) = tx.send(ClientAction::SendChatMessage {
                    message: game_message,
                }) {
                    error!("Failed to send Discord message to game: {}", e);
                } else {
                    info!("Forwarded Discord message to game");
                }
            }
        }
    }
}

impl Handler {
    async fn handle_uptime_command(&self, ctx: &Context, command: &CommandInteraction) {
        let uptime_data = self.uptime_data.read().await;

        let bot_uptime = uptime_data.bot_start.elapsed();
        let bot_secs = bot_uptime.as_secs();
        let bot_hours = bot_secs / 3600;
        let bot_mins = (bot_secs % 3600) / 60;
        let bot_secs_remainder = bot_secs % 60;

        let response_text = if let Some(ingame_start) = uptime_data.ingame_start {
            let ingame_uptime = ingame_start.elapsed();
            let ingame_secs = ingame_uptime.as_secs();
            let ingame_hours = ingame_secs / 3600;
            let ingame_mins = (ingame_secs % 3600) / 60;
            let ingame_secs_remainder = ingame_secs % 60;

            format!(
                "**Bot Uptime:** {:02}:{:02}:{:02}\n**In-Game Time:** {:02}:{:02}:{:02}",
                bot_hours,
                bot_mins,
                bot_secs_remainder,
                ingame_hours,
                ingame_mins,
                ingame_secs_remainder
            )
        } else {
            format!(
                "**Bot Uptime:** {:02}:{:02}:{:02}\n**In-Game Time:** Not logged in yet",
                bot_hours, bot_mins, bot_secs_remainder
            )
        };

        let data = CreateInteractionResponseMessage::new().content(response_text);
        let builder = CreateInteractionResponse::Message(data);

        if let Err(e) = command.create_response(&ctx.http, builder).await {
            error!("Failed to respond to uptime command: {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let _cli = Cli::parse();

    // Get Discord token and channel ID from env vars
    let discord_token = std::env::var("DISCORD_TOKEN").map_err(|_| "DISCORD_TOKEN not provided")?;

    let channel_id_u64 = std::env::var("DISCORD_CHANNEL_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or("DISCORD_CHANNEL_ID not provided")?;

    // Get game server details from env vars
    let game_host = std::env::var("GAME_SERVER_HOST").unwrap_or_else(|_| "localhost".to_string());

    let game_port = std::env::var("GAME_SERVER_PORT").unwrap_or_else(|_| "9000".to_string());

    let game_address = format!("{}:{}", game_host, game_port);

    let game_username = std::env::var("GAME_ACCOUNT").map_err(|_| "GAME_ACCOUNT not provided")?;

    let game_password = std::env::var("GAME_PASSWORD").map_err(|_| "GAME_PASSWORD not provided")?;

    // Create Discord HTTP client for message sending
    let http = Arc::new(serenity::http::Http::new(&discord_token));
    let channel_id = serenity::model::id::ChannelId::new(channel_id_u64);

    // Test Discord connection
    match http.get_user(serenity::model::id::UserId::new(1)).await {
        Ok(_) => {
            info!("Discord authentication successful");
        }
        Err(_) => {
            // We expect this to fail, but it tests if the token is valid
            info!("Discord token validated");
        }
    }

    // Create channels for client communication
    let (_client_event_tx, _client_event_rx) = tokio::sync::mpsc::unbounded_channel::<GameEvent>();
    let (action_tx_channel, mut action_tx_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create shutdown channel
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Create shared uptime data
    let uptime_data = Arc::new(RwLock::new(UptimeData {
        bot_start: Instant::now(),
        ingame_start: None,
    }));

    // Create client configuration
    let config = ClientConfig {
        id: 0,
        address: game_address,
        account_name: game_username,
        password: game_password,
    };

    let event_bus_manager = Arc::new(EventBusManager::new(100));
    
    // Spawn client task
    let http_clone = http.clone();
    let uptime_data_clone = uptime_data.clone();
    let _client_handle = tokio::spawn(gromnie_runner::run_client_with_action_channel(
        config,
        event_bus_manager,
        move |action_tx| {
            DiscordConsumer::new_with_uptime(
                action_tx.clone(),
                http_clone.clone(),
                channel_id,
                uptime_data_clone.clone(),
            )
        },
        action_tx_channel,
        shutdown_rx,
    ));

    // Create Arc<Mutex<>> for action_tx so it can be shared with Discord handler
    let action_tx_arc = Arc::new(tokio::sync::Mutex::new(None));

    // Wait for the action_tx channel from the client task (with timeout)
    match tokio::time::timeout(tokio::time::Duration::from_secs(5), action_tx_rx.recv()).await {
        Ok(Some(action_tx)) => {
            info!("Game client connected");
            *action_tx_arc.lock().await = Some(action_tx);
        }
        _ => {
            error!("Failed to receive action_tx from game client task");
        }
    }

    // Spawn periodic uptime logging task
    let uptime_data_periodic = uptime_data.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            let data = uptime_data_periodic.read().await;
            let total_uptime = data.bot_start.elapsed();
            let total_secs = total_uptime.as_secs();
            let total_hours = total_secs / 3600;
            let total_mins = (total_secs % 3600) / 60;
            let total_secs_remainder = total_secs % 60;

            if let Some(ingame_start) = data.ingame_start {
                let ingame_uptime = ingame_start.elapsed();
                let ingame_secs = ingame_uptime.as_secs();
                let ingame_hours = ingame_secs / 3600;
                let ingame_mins = (ingame_secs % 3600) / 60;
                let ingame_secs_remainder = ingame_secs % 60;
                info!(
                    "Uptime - Bot: {:02}:{:02}:{:02} | In-game: {:02}:{:02}:{:02}",
                    total_hours,
                    total_mins,
                    total_secs_remainder,
                    ingame_hours,
                    ingame_mins,
                    ingame_secs_remainder
                );
            } else {
                info!(
                    "Bot uptime: {:02}:{:02}:{:02} (not in-game yet)",
                    total_hours, total_mins, total_secs_remainder
                );
            }
        }
    });

    info!("Starting Discord bot gateway connection");

    // Create the Discord client
    let mut discord_client = serenity::client::Client::builder(
        &discord_token,
        GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGES,
    )
    .event_handler(Handler {
        target_channel_id: channel_id,
        action_tx: action_tx_arc,
        uptime_data: uptime_data.clone(),
    })
    .await
    .map_err(|e| format!("Failed to create Discord client: {}", e))?;

    // Run client and listen for Ctrl+C
    tokio::select! {
        result = discord_client.start() => {
            match result {
                Ok(_) => info!("Discord client disconnected"),
                Err(e) => error!("Discord client error: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
    }

    info!("Discord bot shut down");
    Ok(())
}
