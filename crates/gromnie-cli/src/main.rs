use std::error::Error;

use clap::Parser;
use gromnie_runner::logging;
use ratatui::{TerminalOptions, Viewport};
use tracing::info;

use gromnie_cli::cli_runtime::{
    build_client_config, load_or_create_config, resolve_reconnect, resolve_server_and_account,
    run_client,
};
use gromnie_cli::{app::App, run as cli_run};

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

    /// Enable automatic reconnection on connection loss
    #[arg(long, conflicts_with = "no_reconnect")]
    reconnect: bool,

    /// Disable automatic reconnection (overrides config file)
    #[arg(long)]
    no_reconnect: bool,
}

const EXAMPLE_CONFIG: &str = r#"# Gromnie Configuration
# Edit this file to add servers and accounts

[servers.local]
host = "localhost"
port = 9000

[accounts.default]
username = "user"
password = "pass"

[scripting]
enabled = true
# Script execution timeout in milliseconds (default: 100ms)
# Prevents misbehaving scripts from blocking the entire scripting system
# script_timeout_ms = 100
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging (opt-in file logging via GROMNIE_LOG_FILE env var)
    // The guard must be kept alive for the entire program duration to ensure logs are flushed
    #[allow(unused_variables)]
    let log_guard = logging::init_logging("cli")?;

    let cli = Cli::parse();

    info!("Starting gromnie client...");

    let Some(config) = load_or_create_config(EXAMPLE_CONFIG)? else {
        return Ok(());
    };

    // If server and account are provided via CLI args, use them directly
    match (cli.server.clone(), cli.account.clone()) {
        (Some(server_name), Some(account_name)) => {
            let (server, account) =
                resolve_server_and_account(&config, &server_name, &account_name)?;
            let client_config = build_client_config(
                server,
                account,
                resolve_reconnect(cli.no_reconnect, cli.reconnect, config.reconnect),
                None,
            );

            info!(
                "Connecting to {} with account {}",
                server_name, account_name
            );

            run_client(client_config, config.clone()).await?;

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

        let client_config = build_client_config(
            server,
            account,
            resolve_reconnect(cli.no_reconnect, cli.reconnect, wizard.config.reconnect),
            None,
        );
        run_client(client_config, wizard.config.clone()).await?;
    }

    Ok(())
}
