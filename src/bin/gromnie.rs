use std::error::Error;

use clap::Parser;
use gromnie::runner::{ClientConfig, LoggingConsumer};
use ratatui::{TerminalOptions, Viewport};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie::cli::{self, App};
use gromnie::config::Config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn run_config_wizard() -> Result<Config, Box<dyn Error>> {
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(20),
    });

    let mut app = App::new();
    let result = cli::run(&mut app, &mut terminal);

    ratatui::restore();
    result?;

    // Extract and save config from completed wizard
    if let Some(wizard) = app.config_wizard {
        let config = wizard.to_config();
        config.save()?;
        info!("Configuration saved to {}", Config::config_path().display());
        Ok(config)
    } else {
        Err("Config wizard incomplete".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let _cli = Cli::parse();

    info!("Starting gromnie client...");

    // Load or create config
    let config = match Config::load() {
        Ok(cfg) => {
            info!("Loaded existing config");
            cfg
        }
        Err(_) => {
            info!("No config found, running config wizard");
            run_config_wizard()?
        }
    };

    // Run the launch wizard
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(12),
    });

    let mut app = App::new_with_config(config);
    let app_result = cli::run(&mut app, &mut terminal);

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
        gromnie::runner::run_client(client_config, LoggingConsumer::new, None).await;
    }

    Ok(())
}
