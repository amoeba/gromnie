use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub username: String,
    pub password: String,
}

impl std::fmt::Display for AccountConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.username)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub servers: BTreeMap<String, ServerConfig>,
    pub accounts: BTreeMap<String, AccountConfig>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            let mut path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
            path.push(".config/gromnie/config.toml");
            path
        }

        #[cfg(not(target_os = "macos"))]
        {
            use directories::ProjectDirs;
            let proj_dirs =
                ProjectDirs::from("", "", "gromnie").expect("Failed to determine config directory");
            proj_dirs.config_dir().join("config.toml")
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();

        if !path.exists() {
            return Err("Config file not found".into());
        }

        let content = fs::read_to_string(&path)?;
        let config = toml::from_str(&content)?;
        info!("Loaded config from {}", path.display());
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(&self)?;
        fs::write(&path, content)?;
        info!("Saved config to {}", path.display());
        Ok(())
    }

    pub fn setup_wizard() -> Result<Self, Box<dyn std::error::Error>> {
        println!("\n=== Gromnie Configuration Wizard ===\n");

        let mut servers = BTreeMap::new();
        let mut accounts = BTreeMap::new();

        // Add first server
        println!("Let's add a server:");
        let server_key = Self::prompt("Server name/key (e.g., 'localhost')")?;
        let server_host = Self::prompt("Server host (e.g., 'localhost')")?;
        let server_port = Self::prompt_with_default("Server port", "9000")?;
        let server_port: u16 = server_port.parse()?;
        servers.insert(
            server_key,
            ServerConfig {
                host: server_host,
                port: server_port,
            },
        );

        let mut config = Config {
            servers: servers.clone(),
            accounts: accounts.clone(),
        };
        config.save()?;
        println!("Server saved!");

        // Ask if they want to add more servers
        loop {
            if Self::prompt_yes_no("Add another server?")? {
                let server_key = Self::prompt("Server name/key")?;
                let server_host = Self::prompt("Server host")?;
                let server_port = Self::prompt_with_default("Server port", "9000")?;
                let server_port: u16 = server_port.parse()?;
                servers.insert(
                    server_key,
                    ServerConfig {
                        host: server_host,
                        port: server_port,
                    },
                );
                config = Config {
                    servers: servers.clone(),
                    accounts: accounts.clone(),
                };
                config.save()?;
                println!("Server saved!");
            } else {
                break;
            }
        }

        // Add first account
        println!("\nLet's add an account:");
        let username = Self::prompt("Username")?;
        let password = Self::prompt("Password")?;
        accounts.insert(username.clone(), AccountConfig { username, password });
        config = Config {
            servers: servers.clone(),
            accounts: accounts.clone(),
        };
        config.save()?;
        println!("Account saved!");

        // Ask if they want to add more accounts
        loop {
            if Self::prompt_yes_no("Add another account?")? {
                let username = Self::prompt("Username")?;
                let password = Self::prompt("Password")?;
                accounts.insert(username.clone(), AccountConfig { username, password });
                config = Config {
                    servers: servers.clone(),
                    accounts: accounts.clone(),
                };
                config.save()?;
                println!("Account saved!");
            } else {
                break;
            }
        }

        println!(
            "\nConfiguration complete and saved to {}\n",
            Self::config_path().display()
        );
        Ok(config)
    }

    fn prompt(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        print!("{}: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn prompt_with_default(
        prompt: &str,
        default: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        print!("{} [{}]: ", prompt, default);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        Ok(if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed.to_string()
        })
    }

    fn prompt_yes_no(prompt: &str) -> Result<bool, Box<dyn std::error::Error>> {
        loop {
            let response = Self::prompt(&format!("{} (y/n)", prompt))?;
            match response.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => println!("Please enter 'y' or 'n'"),
            }
        }
    }

    pub fn select_server(&self) -> Result<ServerConfig, Box<dyn std::error::Error>> {
        if self.servers.is_empty() {
            return Err("No servers configured".into());
        }

        let config_path = Self::config_path();
        let servers: Vec<_> = self.servers.values().cloned().collect();

        println!("Welcome to Gromnie!");
        println!("{}\n", config_path.display());

        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let viewport_height = (1 + servers.len()).min(30) as u16;
        let options = ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Inline(viewport_height),
        };
        let mut terminal = Terminal::with_options(backend, options)?;

        let mut selected = 0;
        let mut confirmed = false;

        loop {
            terminal.draw(|f| {
                let prefix = if confirmed { "  " } else { "? " };
                let mut lines = vec![format!("{}Pick a server", prefix)];
                for (idx, item) in servers.iter().enumerate() {
                    if idx == selected {
                        lines.push(format!("  ● {}", item));
                    } else {
                        lines.push(format!("  ○ {}", item));
                    }
                }
                let text = lines.join("\n");
                let paragraph = Paragraph::new(text);
                f.render_widget(paragraph, f.area());
            })?;

            if confirmed {
                disable_raw_mode()?;
                return Ok(servers[selected].clone());
            }

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up => {
                            if selected > 0 {
                                selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if selected < servers.len() - 1 {
                                selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            confirmed = true;
                        }
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            return Err("Selection cancelled".into());
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            disable_raw_mode()?;
                            return Err("Selection cancelled".into());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn select_account(
        &self,
        server: &ServerConfig,
    ) -> Result<AccountConfig, Box<dyn std::error::Error>> {
        if self.accounts.is_empty() {
            return Err("No accounts configured".into());
        }

        let accounts: Vec<_> = self.accounts.values().cloned().collect();
        let server_display = format!("Server: {}:{}", server.host, server.port);

        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let viewport_height = (2 + accounts.len()).min(30) as u16;
        let options = ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Inline(viewport_height),
        };
        let mut terminal = Terminal::with_options(backend, options)?;

        let mut selected = 0;
        let mut confirmed = false;

        loop {
            terminal.draw(|f| {
                let prefix = if confirmed { "  " } else { "? " };
                let mut lines = vec![
                    format!("  {}", server_display),
                    format!("{}Pick an account", prefix),
                ];
                for (idx, item) in accounts.iter().enumerate() {
                    if idx == selected {
                        lines.push(format!("  ● {}", item));
                    } else {
                        lines.push(format!("  ○ {}", item));
                    }
                }
                let text = lines.join("\n");
                let paragraph = Paragraph::new(text);
                f.render_widget(paragraph, f.area());
            })?;

            if confirmed {
                disable_raw_mode()?;
                return Ok(accounts[selected].clone());
            }

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Up => {
                            if selected > 0 {
                                selected -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if selected < accounts.len() - 1 {
                                selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            confirmed = true;
                        }
                        KeyCode::Esc => {
                            disable_raw_mode()?;
                            return Err("Selection cancelled".into());
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            disable_raw_mode()?;
                            return Err("Selection cancelled".into());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
