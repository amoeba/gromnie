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
    config_wizard: Option<ConfigWizard>,
}
impl App {
    fn new() -> Self {
        Self {
            app_screen: AppScreen::Config,
            launch_wizard: None,
            config_wizard: Some(ConfigWizard::new()),
        }
    }

    fn new_with_config(config: Config) -> Self {
        Self {
            app_screen: AppScreen::Launch,
            launch_wizard: Some(LaunchWizard::new(config)),
            config_wizard: None,
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

        let server_list: Vec<_> = config
            .servers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let account_list: Vec<_> = config
            .accounts
            .iter()
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
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span, Text};
        use ratatui::widgets::Paragraph;

        let mut lines = Vec::new();

        // Always show welcome and config path
        lines.push(Line::from("Welcome to Gromnie!"));
        lines.push(Line::from(Span::styled(
            format!("Config: {}", Config::config_path().display()),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Show server selection based on stage
        match self.stage {
            WizardStage::Welcome => {
                // Just show welcome message
            }
            WizardStage::SelectingServer => {
                lines.push(Line::from("? Pick a server"));
                for (idx, (_name, server)) in self.server_list.iter().enumerate() {
                    let marker = if idx == self.selected_server_idx {
                        "●"
                    } else {
                        "○"
                    };
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
                        let marker = if idx == self.selected_account_idx {
                            "●"
                        } else {
                            "○"
                        };
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

#[derive(Debug, Clone, PartialEq)]
enum ConfigWizardStage {
    Welcome,
    EnteringServerName,
    EnteringServerHost,
    EnteringServerPort,
    EnteringAccountUsername,
    EnteringAccountPassword,
    Confirming,
    Complete,
}

#[derive(Clone)]
struct ConfigWizard {
    stage: ConfigWizardStage,
    server_name: String,
    server_host: String,
    server_port: String,
    account_username: String,
    account_password: String,
    current_input: String,
}

impl ConfigWizard {
    fn new() -> Self {
        Self {
            stage: ConfigWizardStage::Welcome,
            server_name: String::new(),
            server_host: String::new(),
            server_port: String::new(),
            account_username: String::new(),
            account_password: String::new(),
            current_input: String::new(),
        }
    }

    fn to_config(&self) -> Config {
        use std::collections::BTreeMap;

        let mut servers = BTreeMap::new();
        servers.insert(
            self.server_name.clone(),
            gromnie::config::ServerConfig {
                host: self.server_host.clone(),
                port: self.server_port.parse().unwrap_or(9000),
            },
        );

        let mut accounts = BTreeMap::new();
        accounts.insert(
            self.account_username.clone(),
            gromnie::config::AccountConfig {
                username: self.account_username.clone(),
                password: self.account_password.clone(),
            },
        );

        Config { servers, accounts }
    }
}

impl Draw for ConfigWizard {
    fn draw(self: Self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        use ratatui::text::{Line, Text};
        use ratatui::widgets::Paragraph;

        let mut lines = Vec::new();

        // Always show welcome
        lines.push(Line::from("Welcome to Gromnie!"));
        lines.push(Line::from(""));

        match self.stage {
            ConfigWizardStage::Welcome => {
                lines.push(Line::from("No configuration file was found so the following prompts will help you create one."));
                lines.push(Line::from(""));
            }
            _ => {
                lines.push(Line::from("No configuration file was found so the following prompts will help you create one."));
                lines.push(Line::from(""));

                // Show completed server name if we've moved past it
                if self.stage != ConfigWizardStage::EnteringServerName {
                    lines.push(Line::from("  Server Name"));
                    lines.push(Line::from(format!("  {}", self.server_name)));
                    lines.push(Line::from(""));
                }

                // Show server name input if we're on that stage
                if self.stage == ConfigWizardStage::EnteringServerName {
                    lines.push(Line::from(format!(
                        "  Enter a server name: {}█",
                        self.current_input
                    )));
                    lines.push(Line::from(""));
                }

                // Show completed server host if we've moved past it
                if matches!(
                    self.stage,
                    ConfigWizardStage::EnteringServerPort
                        | ConfigWizardStage::EnteringAccountUsername
                        | ConfigWizardStage::EnteringAccountPassword
                        | ConfigWizardStage::Confirming
                        | ConfigWizardStage::Complete
                ) {
                    lines.push(Line::from("  Hostname"));
                    lines.push(Line::from(format!("  {}", self.server_host)));
                    lines.push(Line::from(""));
                }

                // Show server host input if we're on that stage
                if self.stage == ConfigWizardStage::EnteringServerHost {
                    lines.push(Line::from(format!(
                        "  Enter a hostname: {}█",
                        self.current_input
                    )));
                    lines.push(Line::from(""));
                }

                // Show completed port if we've moved past it
                if matches!(
                    self.stage,
                    ConfigWizardStage::EnteringAccountUsername
                        | ConfigWizardStage::EnteringAccountPassword
                        | ConfigWizardStage::Confirming
                        | ConfigWizardStage::Complete
                ) {
                    lines.push(Line::from("  Port"));
                    lines.push(Line::from(format!("  {}", self.server_port)));
                    lines.push(Line::from(""));
                }

                // Show port input if we're on that stage
                if self.stage == ConfigWizardStage::EnteringServerPort {
                    lines.push(Line::from(format!(
                        "  Enter a port (default: 9000): {}█",
                        self.current_input
                    )));
                    lines.push(Line::from(""));
                }

                // Show completed account username if we've moved past it
                if matches!(
                    self.stage,
                    ConfigWizardStage::EnteringAccountPassword
                        | ConfigWizardStage::Confirming
                        | ConfigWizardStage::Complete
                ) {
                    lines.push(Line::from("  Account"));
                    lines.push(Line::from(format!("  {}", self.account_username)));
                    lines.push(Line::from(""));
                }

                // Show username input if we're on that stage
                if self.stage == ConfigWizardStage::EnteringAccountUsername {
                    lines.push(Line::from(format!(
                        "  Enter account name: {}█",
                        self.current_input
                    )));
                    lines.push(Line::from(""));
                }

                // Show completed password if we've moved past it
                if matches!(
                    self.stage,
                    ConfigWizardStage::Confirming | ConfigWizardStage::Complete
                ) {
                    lines.push(Line::from("  Password"));
                    lines.push(Line::from(format!(
                        "  {}",
                        "*".repeat(self.account_password.len())
                    )));
                    lines.push(Line::from(""));
                }

                // Show password input if we're on that stage
                if self.stage == ConfigWizardStage::EnteringAccountPassword {
                    lines.push(Line::from(format!(
                        "  Enter a password: {}█",
                        "*".repeat(self.current_input.len())
                    )));
                    lines.push(Line::from(""));
                }

                // Show confirmation prompt
                if self.stage == ConfigWizardStage::Confirming {
                    lines.push(Line::from(
                        "? Configuration complete. Press <enter> to save",
                    ));
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
    Config,
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
        AppScreen::Config => {
            if let Some(wizard) = &app.config_wizard {
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
            AppScreen::Config => {
                if let Some(wizard) = &mut app.config_wizard {
                    // Auto-advance from Welcome stage
                    if wizard.stage == ConfigWizardStage::Welcome {
                        // std::thread::sleep(std::time::Duration::from_millis(500));
                        wizard.stage = ConfigWizardStage::EnteringServerName;
                        continue;
                    }

                    // Check if wizard is complete
                    if wizard.stage == ConfigWizardStage::Complete {
                        break;
                    }

                    // Poll for keyboard events
                    if event::poll(std::time::Duration::from_millis(16))? {
                        if let Event::Key(key) = event::read()? {
                            match key.code {
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    return Err("Configuration cancelled".into());
                                }
                                KeyCode::Char(c) => {
                                    // Only accept input during text entry stages
                                    if matches!(
                                        wizard.stage,
                                        ConfigWizardStage::EnteringServerName
                                            | ConfigWizardStage::EnteringServerHost
                                            | ConfigWizardStage::EnteringServerPort
                                            | ConfigWizardStage::EnteringAccountUsername
                                            | ConfigWizardStage::EnteringAccountPassword
                                    ) {
                                        wizard.current_input.push(c);
                                    }
                                }
                                KeyCode::Backspace => {
                                    // Remove last character during text entry
                                    if matches!(
                                        wizard.stage,
                                        ConfigWizardStage::EnteringServerName
                                            | ConfigWizardStage::EnteringServerHost
                                            | ConfigWizardStage::EnteringServerPort
                                            | ConfigWizardStage::EnteringAccountUsername
                                            | ConfigWizardStage::EnteringAccountPassword
                                    ) {
                                        wizard.current_input.pop();
                                    }
                                }
                                KeyCode::Enter => match wizard.stage {
                                    ConfigWizardStage::EnteringServerName => {
                                        if !wizard.current_input.is_empty() {
                                            wizard.server_name = wizard.current_input.clone();
                                            wizard.current_input.clear();
                                            wizard.stage = ConfigWizardStage::EnteringServerHost;
                                        }
                                    }
                                    ConfigWizardStage::EnteringServerHost => {
                                        if !wizard.current_input.is_empty() {
                                            wizard.server_host = wizard.current_input.clone();
                                            wizard.current_input.clear();
                                            wizard.stage = ConfigWizardStage::EnteringServerPort;
                                        }
                                    }
                                    ConfigWizardStage::EnteringServerPort => {
                                        wizard.server_port = if wizard.current_input.is_empty() {
                                            "9000".to_string()
                                        } else {
                                            wizard.current_input.clone()
                                        };
                                        wizard.current_input.clear();
                                        wizard.stage = ConfigWizardStage::EnteringAccountUsername;
                                    }
                                    ConfigWizardStage::EnteringAccountUsername => {
                                        if !wizard.current_input.is_empty() {
                                            wizard.account_username = wizard.current_input.clone();
                                            wizard.current_input.clear();
                                            wizard.stage =
                                                ConfigWizardStage::EnteringAccountPassword;
                                        }
                                    }
                                    ConfigWizardStage::EnteringAccountPassword => {
                                        if !wizard.current_input.is_empty() {
                                            wizard.account_password = wizard.current_input.clone();
                                            wizard.current_input.clear();
                                            wizard.stage = ConfigWizardStage::Confirming;
                                        }
                                    }
                                    ConfigWizardStage::Confirming => {
                                        wizard.stage = ConfigWizardStage::Complete;
                                    }
                                    _ => {}
                                },
                                KeyCode::Esc => {
                                    return Err("Configuration cancelled".into());
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    break;
                }
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
                                KeyCode::Up => match wizard.stage {
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
                                },
                                KeyCode::Down => match wizard.stage {
                                    WizardStage::SelectingServer => {
                                        if wizard.selected_server_idx < wizard.server_list.len() - 1
                                        {
                                            wizard.selected_server_idx += 1;
                                        }
                                    }
                                    WizardStage::SelectingAccount => {
                                        if wizard.selected_account_idx
                                            < wizard.account_list.len() - 1
                                        {
                                            wizard.selected_account_idx += 1;
                                        }
                                    }
                                    _ => {}
                                },
                                KeyCode::Enter => match wizard.stage {
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
                                },
                                KeyCode::Esc => {
                                    return Err("Selection cancelled".into());
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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

fn run_config_wizard() -> Result<Config, Box<dyn Error>> {
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(20),
    });

    let mut app = App::new();
    let result = run(&mut app, &mut terminal);

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
