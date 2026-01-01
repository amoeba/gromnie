use std::error::Error;
use std::fs;

use clap::Parser;
use gromnie_runner::{
    ClientConfig, CompositeConsumer, FnConsumerBuilder, LoggingConsumer, RunConfig,
};
use gromnie_scripting_host::create_script_consumer;
use ratatui::{TerminalOptions, Viewport};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie_cli::{app::App, run as cli_run};
use gromnie_client::config::Config;

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
}

fn create_example_config() -> Result<(), Box<dyn Error>> {
    let config_path = Config::config_path();

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
    let config = match Config::load() {
        Ok(cfg) => {
            info!("Loaded existing config");
            cfg
        }
        Err(_) => {
            info!("No config found, creating example config");
            create_example_config()?;
            return Ok(());
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
            };

            info!(
                "Connecting to {} with account {}",
                server_name, account_name
            );

            let run_config = RunConfig::single(client_config);

            // Always use CompositeConsumer - just add scripting consumer if enabled
            let scripting_config = config.scripting.clone();
            let consumer_builder = FnConsumerBuilder::new(
                move |_, _, action_tx| {
                    let mut consumers =
                        vec![Box::new(LoggingConsumer::new(action_tx.clone()))];

                    if let Some(scripting_config) = scripting_config.as_ref() {
                        consumers.push(Box::new(create_script_consumer(action_tx, scripting_config)));
                    }

                    Box::new(CompositeConsumer::new(consumers))
                },
            );

            gromnie_runner::run(
                run_config,
                consumer_builder,
                None::<fn(u32) -> ClientConfig>,
                None,
            )
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
    let app_result = cli_run(&mut app, &mut terminal);
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
        };

        let run_config = RunConfig::single(client_config);

        // Always use CompositeConsumer - just add scripting consumer if enabled
        let scripting_config = wizard.config.scripting.clone();
        let consumer_builder = FnConsumerBuilder::new(
            move |_, _, action_tx| {
                let mut consumers =
                    vec![Box::new(LoggingConsumer::new(action_tx.clone()))];

                if let Some(scripting_config) = scripting_config.as_ref() {
                    consumers.push(Box::new(create_script_consumer(action_tx, scripting_config)));
                }

                Box::new(CompositeConsumer::new(consumers))
            },
        );

        gromnie_runner::run(
            run_config,
            consumer_builder,
            None::<fn(u32) -> ClientConfig>,
            None,
        )
        .await;
    }

    Ok(())
}
