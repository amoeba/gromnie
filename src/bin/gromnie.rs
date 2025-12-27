use std::error::Error;

use clap::Parser;
use ratatui::prelude::Backend;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie::config::Config;
use gromnie::runner::{ClientConfig, LoggingConsumer};

struct App {
    app_screen: AppScreen,
}
impl App {
    fn new() -> Self {
        Self {
            app_screen: AppScreen::Setup,
        }
    }
}
trait Draw {
    fn draw(self: Self, app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>>;
}
struct SetupWizard {}
impl SetupWizard {
    fn new() -> Self {
        Self {}
    }
}
impl Draw for SetupWizard {
    fn draw(self: Self, app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        let area = frame.area();

        let block = Block::new().title(Line::from("Progress").centered());
        frame.render_widget(block, area);

        Ok(())
    }
}
struct LaunchWizard {}
impl LaunchWizard {
    fn new() -> Self {
        Self {}
    }
}
impl Draw for LaunchWizard {
    fn draw(self: Self, app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        let area = frame.area();
        let block = Block::new().title(Line::from("Progress").centered());
        frame.render_widget(block, area);

        Ok(())
    }
}
enum AppScreen {
    Setup,
    Launch,
}
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Enables debug mode
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    match &app.app_screen {
        AppScreen::Setup => {
            SetupWizard::new().draw(app, frame);
        }
        AppScreen::Launch => {
            LaunchWizard::new().draw(app, frame);
        }
    }
}

fn run(app: &mut App, terminal: &mut Terminal<impl Backend>) -> Result<(), Box<dyn Error>> {
    terminal.draw(|frame| draw(app, frame))?;
    // TODO: Input
    Ok(())
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

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(8),
    });

    let mut app = App::new();
    let app_result = run(&mut app, &mut terminal);

    ratatui::restore();

    app_result

    // // Load or create config
    // let config = match Config::load() {
    //     Ok(cfg) => {
    //         info!("Loaded existing config");
    //         cfg
    //     }
    //     Err(_) => {
    //         info!("No config found, running setup wizard");
    //         Config::setup_wizard()?
    //     }
    // };

    // // Let user select server and account
    // let server = config.select_server()?;
    // let account = config.select_account(&server)?;

    // let address = format!("{}:{}", server.host, server.port);
    // info!("Connecting to: {}", address);
    // info!("Account: {}", account.username);

    // // Create client configuration
    // let client_config = ClientConfig {
    //     id: 0,
    //     address,
    //     account_name: account.username,
    //     password: account.password,
    // };

    // // Run the client (this will block until shutdown)
    // gromnie::runner::run_client(client_config, LoggingConsumer::new, None).await;

    // info!("Client shut down cleanly");

    // Ok(())
}
