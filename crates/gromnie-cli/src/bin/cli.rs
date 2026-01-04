use std::error::Error;
use std::fs;

use clap::Parser;
use gromnie_runner::{ClientConfig, ClientRunner, LoggingConsumer};
use ratatui::{TerminalOptions, Viewport};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie_cli::App;
use gromnie_client::config::{ConfigLoadError, GromnieConfig};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Server to connect to
    #[arg(short, long)]
    server: Option<String>,

    /// Account to use
    #[arg(short, long)]
    account: Option<String>,

    /// Character to log in with
    #[arg(short, long)]
    character: Option<String>,

    /// Enable automatic reconnection on connection loss
    #[arg(long, conflicts_with = "no_reconnect")]
    reconnect: bool,

    /// Disable automatic reconnection (overrides config file)
    #[arg(long)]
    no_reconnect: bool,
}

fn create_example_config() -> Result<(), Box<dyn Error>> {
    let config_path = GromnieConfig::config_path();

    // Never overwrite an existing config file
    if config_path.exists() {
        return Err("Config file already exists at {}. Please edit it manually or delete it to create a new one.".into());
    }

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create example config content
    let example_config = r#"# Gromnie Configuration
# Edit this file to add servers and accounts

[servers.local]
host = "localhost"
port = 9000

[accounts.default]
username = "user"
password = "pass"
# Optional: character name to auto-login with
# character = "MyCharacterName"

[scripting]
enabled = true
"#;

    fs::write(&config_path, example_config)?;
    info!("Created example config at {}", config_path.display());
    eprintln!("Config file created at: {}", config_path.display());
    eprintln!("Please edit it with your server and account details, then run gromnie again.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    info!("Starting gromnie client...");

    // Load or create config
    let config = match GromnieConfig::load() {
        Ok(cfg) => {
            info!("Loaded existing config");
            cfg
        }
        Err(ConfigLoadError::NotFound) => {
            info!("No config found, creating example config");
            create_example_config()?;
            return Ok(());
        }
        Err(err) => {
            return Err(format!("Failed to load config: {}", err).into());
        }
    };

    // If server and account are provided via CLI args, use them directly
    match (cli.server.clone(), cli.account.clone()) {
        (Some(server_name), Some(account_name)) => {
            let server = config.servers.get(&server_name).ok_or_else(|| {
                let available = config
                    .servers
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Server '{}' not found. Available servers: {}",
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
                    "Account '{}' not found. Available accounts: {}",
                    account_name, available
                )
            })?;

            let address = format!("{}:{}", server.host, server.port);

            let client_config = ClientConfig {
                id: 0,
                address,
                account_name: account.username.clone(),
                password: account.password.clone(),
                // CLI flags override config file: --reconnect enables, --no-reconnect disables
                reconnect: if cli.no_reconnect {
                    false
                } else if cli.reconnect {
                    true
                } else {
                    config.reconnect
                },
                // CLI flag takes precedence over account config
                character_name: cli.character.clone().or_else(|| account.character.clone()),
            };

            info!(
                "Connecting to {} with account {}{}",
                server_name,
                account_name,
                if let Some(char_name) = &client_config.character_name {
                    format!(", character {}", char_name)
                } else {
                    String::new()
                }
            );

            // Build and run the client using the new builder API
            ClientRunner::builder()
                .with_clients(client_config)
                .with_consumer(LoggingConsumer::from_factory())
                .with_config(config.clone())
                .build()?
                .run()
                .await;

            return Ok(());
        }
        (Some(_), None) => {
            return Err("--server requires --account to be specified".into());
        }
        (None, Some(_)) => {
            return Err("--account requires --server to be specified".into());
        }
        (None, None) => {
            // Run the launch wizard if args not provided
        }
    }
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(12),
    });

    let mut app = App::new_with_config(config);
    let app_result = gromnie_cli::run(&mut app, &mut terminal);
    ratatui::restore();
    app_result?;

    // Extract selected server and account from completed wizard
    if let Some(wizard) = &app.launch_wizard {
        let server = wizard.get_selected_server();
        let account = wizard.get_selected_account();

        let address = format!("{}:{}", server.host, server.port);

        let client_config = ClientConfig {
            id: 0,
            address,
            account_name: account.username.clone(),
            password: account.password.clone(),
            // CLI flags override config file: --reconnect enables, --no-reconnect disables
            reconnect: if cli.no_reconnect {
                false
            } else if cli.reconnect {
                true
            } else {
                wizard.config.reconnect
            },
            character_name: account.character.clone(),
        };

        // Build and run the client using the new builder API
        ClientRunner::builder()
            .with_clients(client_config)
            .with_consumer(LoggingConsumer::from_factory())
            .with_config(wizard.config.clone())
            .build()?
            .run()
            .await;
    }

    Ok(())
}
