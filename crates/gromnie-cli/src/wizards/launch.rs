use ratatui::Frame;
use std::error::Error;

use gromnie_client::config::GromnieConfig;

use crate::app::App;
use crate::draw::Draw;

#[derive(Debug, Clone, PartialEq)]
pub enum WizardStage {
    Welcome,
    SelectingServer,
    SelectingAccount,
    Confirming,
    Complete,
}

#[derive(Clone)]
pub struct LaunchWizard {
    #[allow(dead_code)]
    pub config: GromnieConfig,
    pub stage: WizardStage,
    pub selected_server_idx: usize,
    pub selected_account_idx: usize,
    pub server_list: Vec<(String, gromnie_client::config::ServerConfig)>,
    pub account_list: Vec<(String, gromnie_client::config::AccountConfig)>,
}

impl LaunchWizard {
    pub fn new(config: GromnieConfig) -> Self {
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

    pub fn get_selected_server(&self) -> &gromnie_client::config::ServerConfig {
        &self.server_list[self.selected_server_idx].1
    }

    pub fn get_selected_account(&self) -> &gromnie_client::config::AccountConfig {
        &self.account_list[self.selected_account_idx].1
    }
}

impl Draw for LaunchWizard {
    fn draw(self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span, Text};
        use ratatui::widgets::Paragraph;

        let mut lines = Vec::new();

        // Always show welcome and config path
        lines.push(Line::from("Welcome to Gromnie!"));
        lines.push(Line::from(Span::styled(
            format!("Config: {}", GromnieConfig::config_path().display()),
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
