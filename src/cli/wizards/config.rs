use ratatui::Frame;
use std::error::Error;

use crate::config::Config;

use crate::cli::app::App;
use crate::cli::draw::Draw;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigWizardStage {
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
pub struct ConfigWizard {
    pub stage: ConfigWizardStage,
    pub server_name: String,
    pub server_host: String,
    pub server_port: String,
    pub account_username: String,
    pub account_password: String,
    pub current_input: String,
}

impl ConfigWizard {
    pub fn new() -> Self {
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

    pub fn to_config(&self) -> Config {
        use std::collections::BTreeMap;

        let mut servers = BTreeMap::new();
        servers.insert(
            self.server_name.clone(),
            crate::config::ServerConfig {
                host: self.server_host.clone(),
                port: self.server_port.parse().unwrap_or(9000),
            },
        );

        let mut accounts = BTreeMap::new();
        accounts.insert(
            self.account_username.clone(),
            crate::config::AccountConfig {
                username: self.account_username.clone(),
                password: self.account_password.clone(),
            },
        );

        Config { servers, accounts }
    }
}

impl Draw for ConfigWizard {
    fn draw(self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
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
