use clap::{Parser, Subcommand};
use tracing::error;
use tracing_subscriber::EnvFilter;

use gromnie::client::Client;

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
    let mut client = Client::new(
        id.to_owned(),
        address.to_owned(),
        account_name.to_owned(),
        password.to_owned(),
    )
    .await;

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

    loop {
        match client.socket.recv_from(&mut buf).await {
            Ok((size, peer)) => {
                client.process_packet(&buf[..size], size, &peer).await;

                // Check for and process any messages that were parsed from fragments
                if client.has_messages() {
                    client.process_messages();
                }

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
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Initialize tracing subscriber with env filter
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug"))
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

    for task in tasks {
        task.await.unwrap();
    }

    Ok(())
}
