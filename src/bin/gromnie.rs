use std::error::Error;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::Backend;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};
use tracing::info;
use tracing_subscriber::EnvFilter;

use gromnie::config::Config;

struct App {
    app_screen: AppScreen,
    launch_wizard: Option<LaunchWizard>,
}
impl App {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            app_screen: AppScreen::Setup,
            launch_wizard: None,
        }
    }

    fn new_with_config(config: Config) -> Self {
        Self {
            app_screen: AppScreen::Launch,
            launch_wizard: Some(LaunchWizard::new(config)),
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
    fn draw(self: Self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        let area = frame.area();

        let block = Block::new().title(Line::from("Progress").centered());
        frame.render_widget(block, area);

        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq)]
enum WizardStage {
    Welcome,
    SelectingServer,
    SelectingAccount,
    Confirming,
    Complete,
}

#[derive(Clone)]
struct LaunchWizard {
    #[allow(dead_code)]
    config: Config,
    stage: WizardStage,
    selected_server_idx: usize,
    selected_account_idx: usize,
    server_list: Vec<(String, gromnie::config::ServerConfig)>,
    account_list: Vec<(String, gromnie::config::AccountConfig)>,
}

impl LaunchWizard {
    fn new(config: Config) -> Self {
        // Guard: require at least one server and one account
        assert!(
            !config.servers.is_empty(),
            "Config must have at least one server configured"
        );
        assert!(
            !config.accounts.is_empty(),
            "Config must have at least one account configured"
        );

        let server_list: Vec<_> = config.servers.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let account_list: Vec<_> = config.accounts.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Self {
            config,
            stage: WizardStage::Welcome,
            selected_server_idx: 0,
            selected_account_idx: 0,
            server_list,
            account_list,
        }
    }

    fn get_selected_server(&self) -> &gromnie::config::ServerConfig {
        &self.server_list[self.selected_server_idx].1
    }

    fn get_selected_account(&self) -> &gromnie::config::AccountConfig {
        &self.account_list[self.selected_account_idx].1
    }
}

impl Draw for LaunchWizard {
    fn draw(self: Self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        use ratatui::widgets::Paragraph;
        use ratatui::text::{Line, Span, Text};
        use ratatui::style::{Style, Color};

        let mut lines = Vec::new();

        // Always show welcome and config path
        lines.push(Line::from("Welcome to Gromnie!"));
        lines.push(Line::from(
            Span::styled(
                format!("Config: {}", Config::config_path().display()),
                Style::default().fg(Color::DarkGray)
            )
        ));
        lines.push(Line::from(""));

        // Show server selection based on stage
        match self.stage {
            WizardStage::Welcome => {
                // Just show welcome message
            }
            WizardStage::SelectingServer => {
                lines.push(Line::from("? Pick a server"));
                for (idx, (_name, server)) in self.server_list.iter().enumerate() {
                    let marker = if idx == self.selected_server_idx { "●" } else { "○" };
                    lines.push(Line::from(format!("  {} {}", marker, server)));
                }
            }
            WizardStage::SelectingAccount | WizardStage::Confirming | WizardStage::Complete => {
                // Show completed server selection
                lines.push(Line::from("* Pick a server"));
                let selected_server = &self.server_list[self.selected_server_idx].1;
                lines.push(Line::from(format!("  {}", selected_server)));
                lines.push(Line::from(""));

                if self.stage == WizardStage::SelectingAccount {
                    lines.push(Line::from("? Pick an account"));
                    for (idx, (_name, account)) in self.account_list.iter().enumerate() {
                        let marker = if idx == self.selected_account_idx { "●" } else { "○" };
                        lines.push(Line::from(format!("  {} {}", marker, account)));
                    }
                } else {
                    // Show completed account selection
                    lines.push(Line::from("* Pick an account"));
                    let selected_account = &self.account_list[self.selected_account_idx].1;
                    lines.push(Line::from(format!("  {}", selected_account)));
                    lines.push(Line::from(""));

                    if self.stage == WizardStage::Confirming {
                        lines.push(Line::from("? Ready to log in? <enter>"));
                    }
                }
            }
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text);
        frame.render_widget(paragraph, frame.area());

        Ok(())
    }
}
enum AppScreen {
    #[allow(dead_code)]
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
    match &app.app_screen {
        AppScreen::Setup => {
            let _ = SetupWizard::new().draw(app, frame);
        }
        AppScreen::Launch => {
            if let Some(wizard) = &app.launch_wizard {
                // Clone wizard for drawing (since draw() takes self)
                let _ = wizard.clone().draw(app, frame);
            }
        }
    }
}

fn run(app: &mut App, terminal: &mut Terminal<impl Backend>) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|frame| draw(app, frame))?;

        // Handle input based on current screen
        match &app.app_screen {
            AppScreen::Setup => {
                // TODO: Setup wizard input handling
                break;
            }
            AppScreen::Launch => {
                if let Some(wizard) = &mut app.launch_wizard {
                    // Auto-advance from Welcome stage
                    if wizard.stage == WizardStage::Welcome {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        wizard.stage = WizardStage::SelectingServer;
                        continue;
                    }

                    // Check if wizard is complete
                    if wizard.stage == WizardStage::Complete {
                        break;
                    }

                    // Poll for keyboard events
                    if event::poll(std::time::Duration::from_millis(100))? {
                        if let Event::Key(key) = event::read()? {
                            match key.code {
                                KeyCode::Up => {
                                    match wizard.stage {
                                        WizardStage::SelectingServer => {
                                            if wizard.selected_server_idx > 0 {
                                                wizard.selected_server_idx -= 1;
                                            }
                                        }
                                        WizardStage::SelectingAccount => {
                                            if wizard.selected_account_idx > 0 {
                                                wizard.selected_account_idx -= 1;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                KeyCode::Down => {
                                    match wizard.stage {
                                        WizardStage::SelectingServer => {
                                            if wizard.selected_server_idx < wizard.server_list.len() - 1 {
                                                wizard.selected_server_idx += 1;
                                            }
                                        }
                                        WizardStage::SelectingAccount => {
                                            if wizard.selected_account_idx < wizard.account_list.len() - 1 {
                                                wizard.selected_account_idx += 1;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                KeyCode::Enter => {
                                    match wizard.stage {
                                        WizardStage::SelectingServer => {
                                            wizard.stage = WizardStage::SelectingAccount;
                                        }
                                        WizardStage::SelectingAccount => {
                                            wizard.stage = WizardStage::Confirming;
                                        }
                                        WizardStage::Confirming => {
                                            wizard.stage = WizardStage::Complete;
                                        }
                                        _ => {}
                                    }
                                }
                                KeyCode::Esc => {
                                    return Err("Selection cancelled".into());
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    return Err("Selection cancelled".into());
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }
    }

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

    // Load or create config
    let config = match Config::load() {
        Ok(cfg) => {
            info!("Loaded existing config");
            cfg
        }
        Err(_) => {
            info!("No config found, running setup wizard");
            Config::setup_wizard()?
        }
    };

    // Run the launch wizard
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(12),
    });

    let mut app = App::new_with_config(config);
    let app_result = run(&mut app, &mut terminal);

    ratatui::restore();

    app_result?;

    // Extract selected server and account from completed wizard
    if let Some(wizard) = &app.launch_wizard {
        let server = wizard.get_selected_server();
        let account = wizard.get_selected_account();

        let address = format!("{}:{}", server.host, server.port);
        info!("Selected server: {}", address);
        info!("Selected account: {}", account.username);

        // TODO: Launch the client
        // let client_config = ClientConfig {
        //     id: 0,
        //     address,
        //     account_name: account.username.clone(),
        //     password: account.password.clone(),
        // };
        // gromnie::runner::run_client(client_config, LoggingConsumer::new, None).await;
    }

    Ok(())
}
