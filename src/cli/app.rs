use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::Backend;
use ratatui::{Frame, Terminal};
use std::error::Error;

use crate::config::Config;

use super::draw::Draw;
use super::wizards::{ConfigWizard, ConfigWizardStage, LaunchWizard, WizardStage};

pub struct App {
    pub app_screen: AppScreen,
    pub launch_wizard: Option<LaunchWizard>,
    pub config_wizard: Option<ConfigWizard>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            app_screen: AppScreen::Config,
            launch_wizard: None,
            config_wizard: Some(ConfigWizard::new()),
        }
    }

    pub fn new_with_config(config: Config) -> Self {
        Self {
            app_screen: AppScreen::Launch,
            launch_wizard: Some(LaunchWizard::new(config)),
            config_wizard: None,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        match &self.app_screen {
            AppScreen::Setup => {
                let _ = SetupWizard::new().draw(self, frame);
            }
            AppScreen::Launch => {
                if let Some(wizard) = &self.launch_wizard {
                    let _ = wizard.clone().draw(self, frame);
                }
            }
            AppScreen::Config => {
                if let Some(wizard) = &self.config_wizard {
                    let _ = wizard.clone().draw(self, frame);
                }
            }
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<(), Box<dyn Error>> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            // Handle input based on current screen
            match &self.app_screen {
                AppScreen::Setup => {
                    // TODO: Setup wizard input handling
                    break;
                }
                AppScreen::Config => {
                    if let Some(wizard) = &mut self.config_wizard {
                        // Auto-advance from Welcome stage
                        if wizard.stage == ConfigWizardStage::Welcome {
                            wizard.stage = ConfigWizardStage::EnteringServerName;
                            continue;
                        }

                        // Check if wizard is complete
                        if wizard.stage == ConfigWizardStage::Complete {
                            break;
                        }

                        // Poll for keyboard events
                        if event::poll(std::time::Duration::from_millis(16))?
                            && let Event::Key(key) = event::read()?
                        {
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
                    } else {
                        break;
                    }
                }
                AppScreen::Launch => {
                    if let Some(wizard) = &mut self.launch_wizard {
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
                        if event::poll(std::time::Duration::from_millis(100))?
                            && let Event::Key(key) = event::read()?
                        {
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
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

pub enum AppScreen {
    #[allow(dead_code)]
    Setup,
    Launch,
    Config,
}

struct SetupWizard {}

impl SetupWizard {
    fn new() -> Self {
        Self {}
    }
}

impl Draw for SetupWizard {
    fn draw(self, _app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
        use ratatui::text::Line;
        use ratatui::widgets::Block;

        let area = frame.area();

        let block = Block::new().title(Line::from("Progress").centered());
        frame.render_widget(block, area);

        Ok(())
    }
}
