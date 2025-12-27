use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie::runner::{ClientConfig, LoggingConsumer};

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
        #[arg(short, long, value_name = "USERNAME")]
        username: Option<String>,

        /// Password
        #[arg(short, long, value_name = "PASSWORD")]
        password: Option<String>,
    },
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
    let account_name = "testing";
    let password = "testing";

    info!("Starting gromnie client...");
    info!("Connecting to: {}", address);
    info!("Account: {}", account_name);

    // Create client configuration
    let config = ClientConfig {
        id: 0,
        address: address.to_string(),
        account_name: account_name.to_string(),
        password: password.to_string(),
    };

    // Run the client (this will block until shutdown)
    // The factory function creates the event consumer when the action_tx is available
    gromnie::runner::run_client(config, LoggingConsumer::new, None).await;

    info!("Client shut down cleanly");

    Ok(())
}
