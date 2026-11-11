//! A transcript and a command editor, rather than a game dashboard.
use std::{collections::VecDeque, error::Error, io::Write, path::Path, sync::Arc, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use gromnie_client::{
    api::{ApiError, GromnieClient},
    client::{ConnectingProgress, Scene},
    config::{AccountConfig, ConfigLoadError, GromnieConfig, ServerConfig},
};
use gromnie_events::{ClientEvent, ClientStateEvent, ClientSystemEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use tokio::{sync::broadcast, task::JoinHandle};
use unicode_width::UnicodeWidthStr;

const COMMANDS: &[(&str, &str)] = &[
    ("/account", "select an account or add credentials"),
    ("/server", "select a server or add one"),
    ("/connect", "connect using the active server and account"),
    ("/enter", "pick a character to enter the world"),
    ("/say", "send local chat"),
    ("/tell", "send private chat: /tell Name, message"),
    ("/disconnect", "disconnect and stay here"),
    ("/exit", "leave the game world"),
];

#[derive(Default)]
struct Editor {
    text: String,
    cursor: usize,
}

impl Editor {
    fn set(&mut self, text: String) {
        self.cursor = text.len();
        self.text = text;
    }

    fn key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            KeyCode::Left | KeyCode::Backspace if self.cursor > 0 => {
                let previous = self.text[..self.cursor]
                    .char_indices()
                    .next_back()
                    .unwrap()
                    .0;
                if code == KeyCode::Backspace {
                    self.text.drain(previous..self.cursor);
                }
                self.cursor = previous;
            }
            KeyCode::Right | KeyCode::Delete if self.cursor < self.text.len() => {
                let next =
                    self.cursor + self.text[self.cursor..].chars().next().unwrap().len_utf8();
                if code == KeyCode::Delete {
                    self.text.drain(self.cursor..next);
                } else {
                    self.cursor = next;
                }
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.len(),
            _ => {}
        }
    }
}

enum Prompt {
    Username,
    Password(String),
    ServerName,
    ServerAddress(String),
    ServerPort(String, ServerConfig),
    Delete { account: bool, name: String },
}

enum Outcome {
    Connected(Arc<GromnieClient>),
    Done(String),
}

type Operation = JoinHandle<Result<Outcome, ApiError>>;

struct App {
    config: GromnieConfig,
    account: Option<String>,
    server: Option<String>,
    client: Option<Arc<GromnieClient>>,
    events: Option<broadcast::Receiver<ClientEvent>>,
    operation: Option<Operation>,
    busy: &'static str,
    editor: Editor,
    prompt: Option<Prompt>,
    editing: Option<(bool, String)>,
    transcript: VecDeque<String>,
    selected: usize,
    selection_explicit: bool,
    picker: bool,
    scroll: usize,
    quit: bool,
    quit_confirmation: bool,
}

impl App {
    fn new(config: GromnieConfig) -> Self {
        Self {
            config,
            account: None,
            server: None,
            client: None,
            events: None,
            operation: None,
            busy: "",
            editor: Editor::default(),
            prompt: None,
            editing: None,
            transcript: VecDeque::new(),
            selected: 0,
            selection_explicit: false,
            picker: false,
            scroll: 0,
            quit: false,
            quit_confirmation: false,
        }
    }

    fn push(&mut self, text: impl AsRef<str>) {
        // Server text is untrusted terminal input. Retain newlines, not escapes.
        self.transcript.push_back(
            text.as_ref()
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect(),
        );
        while self.transcript.len() > 1000 {
            self.transcript.pop_front();
        }
    }

    fn characters(&self) -> Vec<String> {
        self.client
            .as_ref()
            .map(|c| c.list_characters().into_iter().map(|c| c.name).collect())
            .unwrap_or_default()
    }

    fn print_characters(&mut self) {
        let characters = self.characters();
        if characters.is_empty() {
            self.push("No enterable characters on this account.");
        } else {
            self.push(format!(
                "Characters:\n  {}\nUse /enter to pick a character.",
                characters.join("\n  ")
            ));
        }
    }

    fn suggestions(&self) -> Vec<(String, String)> {
        if self.prompt.is_some() {
            return vec![];
        }
        let input = &self.editor.text;
        let Some((command, prefix)) = input.split_once(' ') else {
            return if input.starts_with('/') {
                COMMANDS
                    .iter()
                    .filter(|(c, _)| c.starts_with(input))
                    .map(|(c, d)| (c.to_string(), d.to_string()))
                    .collect()
            } else {
                vec![]
            };
        };
        let values = match command {
            "/account" => self
                .config
                .accounts
                .keys()
                .cloned()
                .chain(std::iter::once("new".into()))
                .collect(),
            "/server" => self
                .config
                .servers
                .keys()
                .cloned()
                .chain(std::iter::once("new".into()))
                .collect(),
            "/enter"
                if self
                    .client
                    .as_ref()
                    .is_some_and(|c| matches!(c.scene(), Scene::CharacterSelect(_))) =>
            {
                self.characters()
            }
            _ => vec![],
        };
        values
            .into_iter()
            .filter(|v: &String| v.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|v| {
                let description = match command {
                    "/account" if v == "new" => "add account credentials".into(),
                    "/account" => "switch active account".into(),
                    "/server" if v == "new" => "add a server".into(),
                    "/server" => {
                        let s = &self.config.servers[&v];
                        format!("{}:{}", s.host, s.port)
                    }
                    _ => "enter world".into(),
                };
                (format!("{command} {v}"), description)
            })
            .collect()
    }

    fn complete(&mut self) {
        let suggestions = self.suggestions();
        let exact = (!self.selection_explicit)
            .then(|| {
                suggestions
                    .iter()
                    .position(|(value, _)| value == self.editor.text.trim_end())
            })
            .flatten();
        let selected = exact.unwrap_or(self.selected % suggestions.len().max(1));
        if let Some((value, _)) = suggestions.get(selected) {
            let suffix = if value.contains(' ') { "" } else { " " };
            self.editor.set(format!("{value}{suffix}"));
            self.selected = 0;
            self.selection_explicit = false;
            self.picker = false;
        }
    }

    fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if key.kind == KeyEventKind::Press {
                if self.quit_confirmation {
                    self.quit = true;
                } else {
                    self.quit_confirmation = true;
                    self.push("Press Ctrl-C again to quit. Any other key cancels.");
                }
            }
            return;
        }
        self.quit_confirmation = false;
        match key.code {
            KeyCode::Esc => {
                let picker = match self.prompt.take() {
                    Some(Prompt::Delete { account, .. }) => Some(account),
                    _ => self.editing.take().map(|(account, _)| account),
                };
                self.editing = None;
                self.picker = picker.is_some();
                self.selection_explicit = false;
                self.editor.set(
                    picker
                        .map(|account| if account { "/account " } else { "/server " }.into())
                        .unwrap_or_default(),
                );
                self.selected = 0;
            }
            KeyCode::Char(action @ ('d' | 'e'))
                if self.picker
                    && self.prompt.is_none()
                    && key.modifiers.is_empty()
                    && matches!(self.editor.text.as_str(), "/server " | "/account ") =>
            {
                let account = self.editor.text == "/account ";
                let names: Vec<String> = if account {
                    self.config.accounts.keys().cloned().collect()
                } else {
                    self.config.servers.keys().cloned().collect()
                };
                if let Some(name) = names.get(self.selected) {
                    if action == 'd' {
                        self.confirm_delete(account, name);
                    } else {
                        self.edit_entry(account, name);
                    }
                }
            }
            KeyCode::Tab => self.complete(),
            KeyCode::Up => {
                let n = self.suggestions().len();
                if n > 0 {
                    self.selected = (self.selected + n - 1) % n;
                    self.selection_explicit = true;
                }
            }
            KeyCode::Down => {
                let n = self.suggestions().len();
                if n > 0 {
                    self.selected = (self.selected + 1) % n;
                    self.selection_explicit = true;
                }
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Enter => {
                self.complete();
                self.submit();
            }
            code if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.editor.key(code);
                self.selected = 0;
                self.selection_explicit = false;
                self.picker = false;
            }
            _ => {}
        }
    }

    fn submit(&mut self) {
        let input = std::mem::take(&mut self.editor.text);
        self.editor.cursor = 0;
        self.selected = 0;
        self.selection_explicit = false;
        self.picker = false;
        self.scroll = 0;
        if let Some(prompt) = self.prompt.take() {
            if input.trim().is_empty()
                && !matches!(prompt, Prompt::ServerPort(..) | Prompt::Delete { .. })
                && !(matches!(prompt, Prompt::Password(_))
                    && self.editing.as_ref().is_some_and(|(account, _)| *account))
            {
                self.prompt = Some(prompt);
                self.push("A value is required; Esc cancels.");
                return;
            }
            match prompt {
                Prompt::Username => {
                    let name = input.trim().to_string();
                    if !self.editing.as_ref().is_some_and(|(account, _)| *account)
                        && (name == "new" || self.config.accounts.contains_key(&name))
                    {
                        self.push("That account name already exists or is reserved. Enter another username.");
                        self.prompt = Some(Prompt::Username);
                    } else {
                        self.prompt = Some(Prompt::Password(name));
                    }
                }
                Prompt::Password(name) => {
                    let mut updated = self.config.clone();
                    let editing = self.editing.take();
                    let alias = editing
                        .as_ref()
                        .map(|(_, old)| old.clone())
                        .unwrap_or_else(|| name.clone());
                    let mut password = input;
                    let mut character = None;
                    let mut activate = true;
                    if let Some((true, old)) = &editing {
                        activate = self.account.as_deref() == Some(old.as_str());
                        if let Some(previous) = updated.accounts.remove(old) {
                            character = previous.character;
                            if password.is_empty() {
                                password = previous.password;
                            }
                        }
                    }
                    updated.accounts.insert(
                        alias.clone(),
                        AccountConfig {
                            username: name.clone(),
                            password,
                            character,
                        },
                    );
                    match save_config(&updated) {
                        Ok(()) => {
                            self.config = updated;
                            if activate {
                                self.account = Some(alias.clone());
                            }
                            self.push(format!(
                                "Account saved: {alias}. Credentials saved in {} (plaintext).",
                                GromnieConfig::config_path().display()
                            ));
                            if editing.is_some() {
                                self.editor.set("/account ".into());
                                self.picker = true;
                            }
                        }
                        Err(e) => self.push(format!("Could not save account: {e}")),
                    }
                }
                Prompt::Delete { account, name } => {
                    if !matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                        self.push("Deletion cancelled.");
                    } else {
                        let mut updated = self.config.clone();
                        let clear_active = if account {
                            updated.accounts.remove(&name);
                            self.account.as_deref() == Some(name.as_str())
                        } else {
                            updated.servers.remove(&name).is_some_and(|server| {
                                self.server.as_deref()
                                    == Some(format!("{}:{}", server.host, server.port).as_str())
                            })
                        };
                        match save_config(&updated) {
                            Ok(()) => {
                                self.config = updated;
                                if clear_active {
                                    if account {
                                        self.account = None;
                                    } else {
                                        self.server = None;
                                    }
                                }
                                self.push(format!(
                                    "Deleted {}: {name}",
                                    if account { "account" } else { "server" }
                                ));
                            }
                            Err(e) => self.push(format!("Could not delete: {e}")),
                        }
                    }
                    self.editor
                        .set(if account { "/account " } else { "/server " }.into());
                    self.picker = true;
                }
                Prompt::ServerName => {
                    let name = input.trim().to_string();
                    if name == "new"
                        || (self.config.servers.contains_key(&name)
                            && !self
                                .editing
                                .as_ref()
                                .is_some_and(|(account, old)| !*account && old == &name))
                    {
                        self.push(
                            "That server name already exists or is reserved. Enter another name.",
                        );
                        self.prompt = Some(Prompt::ServerName);
                    } else {
                        if let Some((false, old)) = &self.editing {
                            let server = &self.config.servers[old];
                            self.editor.set(format!("{}:{}", server.host, server.port));
                        }
                        self.prompt = Some(Prompt::ServerAddress(name));
                    }
                }
                Prompt::ServerAddress(name) => match parse_server(input.trim()) {
                    Ok(server) => {
                        self.editor.set(server.port.to_string());
                        self.prompt = Some(Prompt::ServerPort(name, server));
                    }
                    Err(message) => {
                        self.push(message);
                        self.prompt = Some(Prompt::ServerAddress(name));
                    }
                },
                Prompt::ServerPort(name, mut server) => match parse_port(input.trim()) {
                    Ok(port) => {
                        server.port = port;
                        let address = format!("{}:{}", server.host, server.port);
                        let mut updated = self.config.clone();
                        let editing = self.editing.take();
                        let mut activate = true;
                        if let Some((false, old)) = &editing {
                            activate = updated.servers.remove(old).is_some_and(|previous| {
                                self.server.as_deref()
                                    == Some(format!("{}:{}", previous.host, previous.port).as_str())
                            });
                        }
                        updated.servers.insert(name.clone(), server);
                        match save_config(&updated) {
                            Ok(()) => {
                                self.config = updated;
                                if activate {
                                    self.server = Some(address.clone());
                                }
                                self.push(format!("Server saved: {name} ({address})."));
                                if editing.is_some() {
                                    self.editor.set("/server ".into());
                                    self.picker = true;
                                }
                            }
                            Err(e) => self.push(format!("Could not save server: {e}")),
                        }
                    }
                    Err(message) => {
                        self.push(message);
                        self.prompt = Some(Prompt::ServerPort(name, server));
                    }
                },
            }
            return;
        }
        let input = input.trim();
        if input.is_empty() {
            return;
        }
        let (command, argument) = input.split_once(' ').unwrap_or((input, ""));
        let argument = argument.trim();
        match command {
            "/exit" if argument.is_empty() => {
                if let Some(client) = self
                    .client
                    .as_ref()
                    .filter(|c| matches!(c.scene(), Scene::InWorld(_)))
                {
                    match client.log_off() {
                        Ok(()) => self.push("Leaving the game world…"),
                        Err(e) => self.push(format!("Error: {e}")),
                    }
                } else {
                    self.push("Not in the game world.");
                }
            }
            "/disconnect" if argument.is_empty() => self.disconnect(),
            "/account" => {
                if self.client.is_some() || self.operation.is_some() {
                    self.push("/disconnect before switching accounts.");
                } else if argument == "new"
                    || (argument.is_empty() && self.config.accounts.is_empty())
                {
                    self.prompt = Some(Prompt::Username);
                } else if self.config.accounts.contains_key(argument) {
                    self.account = Some(argument.into());
                    self.push(format!("Active account: {argument}"));
                } else {
                    self.editor.set("/account ".into());
                    self.picker = true;
                    self.push("↑/↓ select · Enter activate · e edit · d delete. Use /account new to add credentials.");
                }
            }
            "/server" => {
                if self.client.is_some() || self.operation.is_some() {
                    self.push("/disconnect before switching servers.");
                } else if argument == "new"
                    || (argument.is_empty() && self.config.servers.is_empty())
                {
                    self.prompt = Some(Prompt::ServerName);
                } else if let Some(server) = self.config.servers.get(argument) {
                    let address = format!("{}:{}", server.host, server.port);
                    self.server = Some(address.clone());
                    self.push(format!("Active server: {argument} ({address})"));
                } else {
                    self.editor.set("/server ".into());
                    self.picker = true;
                    self.push("↑/↓ select · Enter activate · e edit · d delete. Use /server new to add one.");
                }
            }
            "/connect" if argument.is_empty() => self.connect(),
            "/connect" => {
                self.push("/connect takes no arguments. Select /server and /account first.")
            }
            "/enter" => {
                if self.operation.is_some() {
                    self.push("Wait for the current operation, or /disconnect.");
                    return;
                }
                let Some(client) = self.client.clone() else {
                    self.push("Use /connect first.");
                    return;
                };
                if !matches!(client.scene(), Scene::CharacterSelect(_)) {
                    self.push("Not at character select.");
                } else if argument.is_empty() {
                    self.editor.set("/enter ".into());
                    self.push(if self.characters().is_empty() {
                        "This account has no enterable characters."
                    } else {
                        "Choose a character with ↑/↓, then Enter."
                    });
                } else {
                    let name = argument.to_string();
                    self.push(format!("Entering as {name}…"));
                    self.busy = "entering world";
                    self.operation = Some(tokio::spawn(async move {
                        client.enter_world(&name).await?;
                        Ok(Outcome::Done(format!("In world as {name}.")))
                    }));
                }
            }
            "/say" | "/tell" => {
                if self.operation.is_some() {
                    self.push("Wait for the current operation, or /disconnect.");
                    return;
                }
                let Some(client) = self
                    .client
                    .clone()
                    .filter(|c| matches!(c.scene(), Scene::InWorld(_)))
                else {
                    self.push("Use /enter before sending chat.");
                    return;
                };
                if argument.is_empty() {
                    self.push("Use /say message or /tell Character Name, message.");
                    return;
                }
                let recipient_message = if command == "/tell" {
                    match argument.split_once(',') {
                        Some((name, message))
                            if !name.trim().is_empty() && !message.trim().is_empty() =>
                        {
                            Some((name.trim().to_string(), message.trim().to_string()))
                        }
                        _ => {
                            self.push("Use /tell Character Name, message.");
                            return;
                        }
                    }
                } else {
                    None
                };
                let message = argument.to_string();
                self.busy = "sending";
                self.operation = Some(tokio::spawn(async move {
                    let echo = if let Some((name, message)) = recipient_message {
                        client.tell(&name, &message).await?;
                        format!("You tell {name}: {message}")
                    } else {
                        client.say(&message).await?;
                        format!("You say: {message}")
                    };
                    Ok(Outcome::Done(echo))
                }));
            }
            _ => self.push("Unknown command or arguments. Type / for commands."),
        }
    }

    fn edit_entry(&mut self, account: bool, name: &str) {
        if self.client.is_some() || self.operation.is_some() {
            self.push("/disconnect before editing saved entries.");
            return;
        }
        self.picker = false;
        self.editing = Some((account, name.into()));
        self.selected = 0;
        if account {
            self.editor.set(self.config.accounts[name].username.clone());
            self.prompt = Some(Prompt::Username);
        } else {
            self.editor.set(name.into());
            self.prompt = Some(Prompt::ServerName);
        }
    }

    fn confirm_delete(&mut self, account: bool, name: &str) {
        if self.client.is_some() || self.operation.is_some() {
            self.push("/disconnect before deleting saved entries.");
            return;
        }
        self.picker = false;
        self.editor.set(String::new());
        self.prompt = Some(Prompt::Delete {
            account,
            name: name.into(),
        });
    }

    fn connect(&mut self) {
        if self.client.is_some() || self.operation.is_some() {
            self.push("Already connected or busy. Use /disconnect first.");
            return;
        }
        let Some(account) = self
            .account
            .as_ref()
            .and_then(|n| self.config.accounts.get(n))
            .cloned()
        else {
            self.push("Use /account first.");
            return;
        };
        let Some(address) = self.server.clone() else {
            self.editor.set("/server ".into());
            self.picker = true;
            self.push("Use /server to select or add a server first.");
            return;
        };
        if let Err(message) = parse_server(&address) {
            self.push(message);
            return;
        }
        let reconnect = self.config.reconnect;
        self.push(format!("Connecting to {address}…"));
        self.busy = "connecting";
        self.operation = Some(tokio::spawn(async move {
            let client = GromnieClient::builder()
                .with_server(address)
                .with_account(account.username, account.password)
                .with_reconnect(reconnect)
                .connect()
                .await?;
            Ok(Outcome::Connected(Arc::new(client)))
        }));
    }

    fn disconnect(&mut self) {
        if let Some(operation) = self.operation.take() {
            operation.abort();
        }
        self.events = None;
        if let Some(client) = self.client.take() {
            self.busy = "disconnecting";
            self.operation = Some(tokio::spawn(async move {
                client.disconnect().await?;
                Ok(Outcome::Done("Disconnected.".into()))
            }));
        } else {
            self.push("Disconnected.");
        }
    }

    async fn tick(&mut self) {
        if self.operation.as_ref().is_some_and(|h| h.is_finished()) {
            match self.operation.take().unwrap().await {
                Ok(Ok(Outcome::Connected(client))) => {
                    self.events = Some(client.subscribe());
                    self.client = Some(client);
                    self.push("Connected.");
                    self.print_characters();
                }
                Ok(Ok(Outcome::Done(message))) => self.push(message),
                Ok(Err(e)) => self.push(format!("Error: {e}")),
                Err(e) => self.push(format!("Operation stopped: {e}")),
            }
        }
        // Bound work per frame so a busy server cannot starve keyboard input.
        for _ in 0..256 {
            let Some(events) = self.events.as_mut() else {
                break;
            };
            match events.try_recv() {
                Ok(ClientEvent::Protocol(event)) => {
                    if let Some(chat) = event.as_chat_message() {
                        self.push(chat.text);
                    }
                }
                Ok(ClientEvent::State(ClientStateEvent::CharacterSelect)) => {
                    self.print_characters();
                }
                Ok(ClientEvent::System(ClientSystemEvent::Disconnected {
                    will_reconnect, ..
                })) => {
                    self.push(if will_reconnect {
                        "Connection lost. Reconnecting…"
                    } else {
                        "Connection lost. Use /disconnect, then /connect to retry."
                    });
                }
                Ok(ClientEvent::System(ClientSystemEvent::AuthenticationFailed { reason })) => {
                    self.push(format!("Authentication failed: {reason}"))
                }
                Ok(_) => {}
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    self.push(format!("Missed {n} events."))
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => {
                    self.events = None;
                    self.push("Connection event stream closed.");
                    break;
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let suggestions = self.suggestions();
        let suggestion_height = suggestions.len().min(8) as u16;
        let areas = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(suggestion_height),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());
        let scene = self.client.as_ref().map(|c| c.scene());
        let (status, character) = status_details(
            scene.as_ref(),
            self.operation.is_some() && self.busy == "connecting",
        );
        frame.render_widget(
            Paragraph::new(format!(
                "{status} · {} · {} · {character}",
                self.server.as_deref().unwrap_or("—"),
                self.account.as_deref().unwrap_or("—")
            ))
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
            areas[3],
        );
        let width = areas[0].width.max(1) as usize;
        let transcript: Vec<&str> = if self.transcript.is_empty() {
            vec![
                "gromnie · no account · no server · disconnected",
                "gromnie — /account → /connect → /enter. Type / for commands.",
                "Tab completes · ↑/↓ select · Enter submits · Esc cancels · PgUp/PgDn scroll · Ctrl-C twice quits",
            ]
        } else {
            self.transcript.iter().map(String::as_str).collect()
        };
        let lines: Vec<Line> = transcript
            .iter()
            .flat_map(|line| {
                line.split('\n')
                    .flat_map(|line| {
                        textwrap::wrap(line, width)
                            .into_iter()
                            .map(|s| Line::raw(s.into_owned()))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let end = lines.len().saturating_sub(
            self.scroll
                .min(lines.len().saturating_sub(areas[0].height as usize)),
        );
        let start = end.saturating_sub(areas[0].height as usize);
        frame.render_widget(Paragraph::new(lines[start..end].to_vec()), areas[0]);
        let selected = self.selected % suggestions.len().max(1);
        let start = selected.saturating_sub(suggestion_height.saturating_sub(1) as usize);
        let command_width = suggestions
            .iter()
            .map(|(value, _)| value.width())
            .max()
            .unwrap_or(0);
        let choices: Vec<Line> = suggestions
            .iter()
            .enumerate()
            .skip(start)
            .take(suggestion_height as usize)
            .map(|(i, (value, description))| {
                Line::styled(
                    format!(
                        "{} {value}{}  {description}",
                        if selected == i { "›" } else { " " },
                        " ".repeat(command_width - value.width())
                    ),
                    Style::default().fg(if selected == i {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    }),
                )
            })
            .collect();
        frame.render_widget(Paragraph::new(choices), areas[1]);
        let title: String = match &self.prompt {
            Some(Prompt::Username) => "Username (Esc cancels)".into(),
            Some(Prompt::Password(_)) if self.editing.is_some() => {
                "Password (Enter keeps existing; Esc cancels)".into()
            }
            Some(Prompt::Password(_)) => {
                "Password (saved locally in plaintext; Esc cancels)".into()
            }
            Some(Prompt::ServerName) => "Server name (Esc cancels)".into(),
            Some(Prompt::ServerAddress(_)) => "Server host (Esc cancels)".into(),
            Some(Prompt::ServerPort(..)) => {
                "Server port [9000] (Enter accepts; Esc cancels)".into()
            }
            Some(Prompt::Delete { account, name }) => format!(
                "Delete {} {name}? [y/N] · Enter submit · Esc cancel",
                if *account { "account" } else { "server" },
            ),
            None => "Command".into(),
        };
        let secret = matches!(self.prompt, Some(Prompt::Password(_)));
        let text = if secret {
            "•".repeat(self.editor.text.chars().count())
        } else {
            self.editor.text.clone()
        };
        let cursor = if secret {
            self.editor.text[..self.editor.cursor].chars().count()
        } else {
            self.editor.text[..self.editor.cursor].width()
        };
        let available = areas[2].width.saturating_sub(2) as usize;
        let offset = cursor.saturating_sub(available.saturating_sub(1));
        frame.render_widget(
            Paragraph::new(text)
                .scroll((0, offset.min(u16::MAX as usize) as u16))
                .block(
                    Block::default()
                        .borders(Borders::TOP | Borders::BOTTOM)
                        .title(title)
                        .border_style(Style::default().fg(Color::Cyan)),
                ),
            areas[2],
        );
        if available > 0 && areas[2].height >= 3 {
            frame.set_cursor_position((areas[2].x + (cursor - offset) as u16, areas[2].y + 1));
        }
    }
}

fn status_details(scene: Option<&Scene>, connecting: bool) -> (&str, &str) {
    match scene {
        Some(Scene::InWorld(world)) => ("in world", &world.character_name),
        Some(Scene::CharacterSelect(select)) => (
            "at char select",
            select
                .entering_world
                .as_ref()
                .map(|c| c.character_name.as_str())
                .unwrap_or("—"),
        ),
        Some(Scene::Connecting(scene)) => match scene.connect_progress {
            ConnectingProgress::Initial | ConnectingProgress::LoginRequestSent => {
                ("connecting", "—")
            }
            _ => ("connected", "—"),
        },
        Some(Scene::CharacterCreate(_)) => ("connected", "—"),
        None if connecting => ("connecting", "—"),
        _ => ("offline", "—"),
    }
}

fn parse_port(input: &str) -> Result<u16, &'static str> {
    if input.is_empty() {
        return Ok(9000);
    }
    input
        .parse::<u16>()
        .ok()
        .filter(|port| (1..u16::MAX).contains(port))
        .ok_or("Enter a port from 1 to 65534, or press Enter for 9000.")
}

fn parse_server(address: &str) -> Result<ServerConfig, &'static str> {
    let invalid = "Enter a hostname or IPv4 address, with a port from 1 to 65534.";
    let (host, port) = match address.rsplit_once(':') {
        Some((host, port)) => (host, parse_port(port)?),
        None => (address, 9000),
    };
    // The native client uses IPv4 and derives a world port as login port + 1.
    if host.is_empty()
        || host
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '/' | ':' | '[' | ']'))
    {
        return Err(invalid);
    }
    Ok(ServerConfig {
        host: host.into(),
        port,
    })
}

fn save_config(config: &GromnieConfig) -> Result<(), Box<dyn Error>> {
    save_config_at(config, &GromnieConfig::config_path())
}

fn save_config_at(config: &GromnieConfig, path: &Path) -> Result<(), Box<dyn Error>> {
    let content = toml::to_string_pretty(config)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    // NamedTempFile is owner-only on Unix. A failed write leaves the original
    // config intact; persist atomically replaces it only after the write succeeds.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

/// Launch the fullscreen client, including on a machine with no config yet.
pub async fn run() -> Result<(), Box<dyn Error>> {
    let config = match GromnieConfig::load() {
        Ok(config) => config,
        Err(ConfigLoadError::NotFound) => GromnieConfig::default(),
        Err(e) => return Err(e.into()),
    };
    let mut app = App::new(config);
    let mut terminal = ratatui::try_init()?;
    // Restore on both I/O errors and ordinary exit (ratatui handles panics).
    let result: std::io::Result<()> = async {
        while !app.quit {
            app.tick().await;
            terminal.draw(|frame| app.draw(frame))?;
            // Do not block the async executor on terminal input.
            for _ in 0..64 {
                if !event::poll(Duration::ZERO)? {
                    break;
                }
                if let Event::Key(key) = event::read()? {
                    app.key(key);
                }
                if app.quit {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(33)).await;
        }
        Ok(())
    }
    .await;
    ratatui::restore();
    if let Some(operation) = app.operation.take() {
        operation.abort();
        let _ = operation.await;
    }
    if let Some(client) = app.client.take() {
        client.disconnect().await?;
    }
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn submit(app: &mut App, text: &str) {
        app.editor.set(text.into());
        app.submit();
    }

    #[test]
    fn command_completion_and_account_flow() {
        let mut app = App::new(GromnieConfig::default());
        app.editor.set("/".into());
        assert_eq!(app.suggestions().len(), COMMANDS.len());
        app.editor.set("/con".into());
        app.complete();
        assert_eq!(app.editor.text, "/connect ");
        submit(&mut app, "/account");
        assert!(matches!(app.prompt, Some(Prompt::Username)));
        submit(&mut app, "tester");
        assert!(matches!(app.prompt, Some(Prompt::Password(ref n)) if n == "tester"));
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.prompt.is_none());
        assert!(app.config.accounts.is_empty());
    }

    #[test]
    fn contextual_completion_preserves_multiword_names() {
        let mut config = GromnieConfig::default();
        config.accounts.insert(
            "Test Account".into(),
            AccountConfig {
                username: "test".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        app.editor.set("/account te".into());
        app.complete();
        assert_eq!(app.editor.text, "/account Test Account");
        app.submit();
        assert_eq!(app.account.as_deref(), Some("Test Account"));
    }

    #[test]
    fn enter_accepts_and_submits_the_highlighted_argument() {
        let mut config = GromnieConfig::default();
        for name in ["First Account", "Second Account"] {
            config.accounts.insert(
                name.into(),
                AccountConfig {
                    username: name.into(),
                    password: "secret".into(),
                    character: None,
                },
            );
        }
        let mut app = App::new(config);
        app.editor.set("/account ".into());
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.account.as_deref(), Some("Second Account"));
        assert!(app.editor.text.is_empty());

        // Tab still completes without submitting.
        app.editor.set("/account Fi".into());
        app.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "/account First Account");
        assert_eq!(app.account.as_deref(), Some("Second Account"));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.account.as_deref(), Some("First Account"));
    }

    #[test]
    fn enter_without_arguments_opens_the_picker() {
        let mut config = GromnieConfig::default();
        config.accounts.insert(
            "tester".into(),
            AccountConfig {
                username: "tester".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        app.editor.set("/account".into());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "/account ");
        assert!(app.account.is_none());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.account.as_deref(), Some("tester"));
    }

    #[test]
    fn server_creation_validates_input_and_can_be_cancelled() {
        let mut app = App::new(GromnieConfig::default());
        submit(&mut app, "/server");
        assert!(matches!(app.prompt, Some(Prompt::ServerName)));
        submit(&mut app, "new");
        assert!(matches!(app.prompt, Some(Prompt::ServerName)));
        submit(&mut app, "Coldeve");
        assert!(matches!(app.prompt, Some(Prompt::ServerAddress(ref name)) if name == "Coldeve"));
        submit(&mut app, "play.coldeve.ac:invalid");
        assert!(matches!(app.prompt, Some(Prompt::ServerAddress(_))));
        submit(&mut app, "play.coldeve.ac");
        assert!(
            matches!(app.prompt, Some(Prompt::ServerPort(ref name, ref server)) if name == "Coldeve" && server.host == "play.coldeve.ac")
        );
        assert_eq!(app.editor.text, "9000");
        submit(&mut app, "invalid");
        assert!(matches!(app.prompt, Some(Prompt::ServerPort(..))));
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.prompt.is_none());
        assert!(app.config.servers.is_empty());
        assert!(app.server.is_none());
    }

    #[test]
    fn picker_delete_uses_highlighted_entry_and_requires_confirmation() {
        let mut config = GromnieConfig::default();
        config.servers.insert(
            "First".into(),
            ServerConfig {
                host: "localhost".into(),
                port: 9000,
            },
        );
        config.servers.insert(
            "Second".into(),
            ServerConfig {
                host: "example.com".into(),
                port: 9000,
            },
        );
        config.accounts.insert(
            "tester".into(),
            AccountConfig {
                username: "tester".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        submit(&mut app, "/server");
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(
            matches!(app.prompt, Some(Prompt::Delete { account: false, ref name }) if name == "Second")
        );
        assert!(app.editor.text.is_empty());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.prompt.is_none());
        assert_eq!(app.editor.text, "/server ");
        assert_eq!(app.config.servers.len(), 2);
        submit(&mut app, "/account");
        app.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(
            matches!(app.prompt, Some(Prompt::Delete { account: true, ref name }) if name == "tester")
        );
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "/account ");
        assert_eq!(app.config.accounts.len(), 1);
    }

    #[test]
    fn deletion_confirmation_is_next_to_input_not_in_transcript() {
        let mut app = App::new(GromnieConfig::default());
        app.push("Previous output");
        app.confirm_delete(true, "treestats");
        assert_eq!(app.transcript.len(), 1);
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content
            .chunks(100)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect();
        assert!(rows[20].contains("Delete account treestats? [y/N]"));
        assert!(!rows[..20].iter().any(|row| row.contains("Delete account")));
        assert_eq!(terminal.get_cursor_position().unwrap().y, 21);
    }

    #[test]
    fn picker_edit_prefills_fields_and_escape_preserves_config() {
        let mut config = GromnieConfig::default();
        config.servers.insert(
            "Local".into(),
            ServerConfig {
                host: "localhost".into(),
                port: 1234,
            },
        );
        config.accounts.insert(
            "tester".into(),
            AccountConfig {
                username: "tester".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        submit(&mut app, "/server");
        app.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(matches!(app.prompt, Some(Prompt::ServerName)));
        assert_eq!(app.editor.text, "Local");
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "localhost:1234");
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "1234");
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.editing.is_none());
        assert_eq!(app.editor.text, "/server ");
        assert_eq!(app.config.servers["Local"].port, 1234);
        submit(&mut app, "/account");
        app.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(app.editor.text, "tester");
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.prompt, Some(Prompt::Password(_))));
        assert!(app.editor.text.is_empty());
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.editor.text, "/account ");
        assert_eq!(app.config.accounts["tester"].password, "secret");
    }

    #[test]
    fn typing_argument_names_does_not_trigger_picker_shortcuts() {
        let mut app = App::new(GromnieConfig::default());
        for command in ["/account default", "/server example"] {
            app.editor.set(String::new());
            for c in command.chars() {
                app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
            assert_eq!(app.editor.text, command);
            assert!(app.prompt.is_none());
        }
    }

    #[test]
    fn exact_completion_wins_unless_user_explicitly_navigates() {
        let mut config = GromnieConfig::default();
        config.accounts.insert(
            "newbie".into(),
            AccountConfig {
                username: "newbie".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        app.editor.set("/account new".into());
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.prompt, Some(Prompt::Username)));
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.editor.set("/account new".into());
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.account.as_deref(), Some("newbie"));
    }

    #[test]
    fn editing_username_does_not_validate_it_as_an_alias() {
        let mut config = GromnieConfig::default();
        for alias in ["default", "user"] {
            config.accounts.insert(
                alias.into(),
                AccountConfig {
                    username: "user".into(),
                    password: "secret".into(),
                    character: None,
                },
            );
        }
        let mut app = App::new(config);
        submit(&mut app, "/account");
        app.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(app.editor.text, "user");
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.prompt, Some(Prompt::Password(ref name)) if name == "user"));
        assert_eq!(app.editing, Some((true, "default".into())));
    }

    #[test]
    fn config_replacement_is_atomic_and_private() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "previous contents").unwrap();
        let mut config = GromnieConfig::default();
        config.accounts.insert(
            "default".into(),
            AccountConfig {
                username: "user".into(),
                password: "secret".into(),
                character: None,
            },
        );
        save_config_at(&config, &path).unwrap();
        let loaded: GromnieConfig =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.accounts["default"].password, "secret");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // A failed replacement must not damage an existing target or leave a temporary file.
        let target = directory.path().join("existing-directory");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("original"), "keep me").unwrap();
        assert!(save_config_at(&config, &target).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("original")).unwrap(),
            "keep me"
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[tokio::test]
    async fn reconnect_notification_respects_the_event() {
        let mut app = App::new(GromnieConfig::default());
        let (tx, rx) = broadcast::channel(8);
        app.events = Some(rx);
        tx.send(ClientEvent::System(ClientSystemEvent::Disconnected {
            will_reconnect: true,
            reconnect_attempt: 1,
            delay_secs: 1,
        }))
        .unwrap();
        app.tick().await;
        assert_eq!(
            app.transcript.back().unwrap(),
            "Connection lost. Reconnecting…"
        );
    }

    #[test]
    fn invalid_saved_server_is_rejected_before_connecting() {
        let mut config = GromnieConfig::default();
        config.accounts.insert(
            "default".into(),
            AccountConfig {
                username: "user".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        app.account = Some("default".into());
        for address in ["localhost:65535", "[::1]:9000"] {
            app.server = Some(address.into());
            submit(&mut app, "/connect");
            assert!(app.operation.is_none());
        }
    }

    #[test]
    fn blank_port_accepts_default() {
        assert_eq!(parse_port(""), Ok(9000));
        assert_eq!(parse_port("9000"), Ok(9000));
        assert_eq!(parse_port("1234"), Ok(1234));
        for invalid in ["0", "65535", "65536", "invalid"] {
            assert!(parse_port(invalid).is_err());
        }
    }

    #[test]
    fn server_addresses_accept_hosts_and_validate_ports() {
        let server = parse_server("play.coldeve.ac").unwrap();
        assert_eq!(server.host, "play.coldeve.ac");
        assert_eq!(server.port, 9000);
        let server = parse_server("localhost:1234").unwrap();
        assert_eq!(server.port, 1234);
        assert!(parse_server("[::1]:9000").is_err());
        assert_eq!(parse_server("localhost:65534").unwrap().port, 65534);
        for invalid in [
            "",
            ":9000",
            "host:0",
            "host:65535",
            "host:65536",
            "::1",
            "[::1]",
            "host:no",
            "bad host:9000",
            "https://host:9000",
        ] {
            assert!(parse_server(invalid).is_err(), "{invalid}");
        }
    }

    #[tokio::test]
    async fn server_selection_is_used_by_connect_and_survives_disconnect() {
        let mut config = GromnieConfig::default();
        config.servers.insert(
            "Local Test".into(),
            ServerConfig {
                host: "localhost".into(),
                port: 9000,
            },
        );
        config.accounts.insert(
            "tester".into(),
            AccountConfig {
                username: "tester".into(),
                password: "secret".into(),
                character: None,
            },
        );
        let mut app = App::new(config);
        app.editor.set("/server Lo".into());
        assert_eq!(
            app.suggestions(),
            vec![("/server Local Test".into(), "localhost:9000".into())]
        );
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.server.as_deref(), Some("localhost:9000"));
        submit(&mut app, "/account tester");
        submit(&mut app, "/connect");
        assert!(app.operation.is_some());
        assert!(
            app.transcript
                .back()
                .unwrap()
                .contains("Connecting to localhost:9000")
        );
        submit(&mut app, "/server new");
        assert!(app.prompt.is_none());
        assert!(app.transcript.back().unwrap().contains("/disconnect"));
        // Cancel before yielding, so this test never opens a network connection.
        submit(&mut app, "/disconnect");
        assert_eq!(app.server.as_deref(), Some("localhost:9000"));
    }

    #[test]
    fn editor_handles_unicode_and_middle_edits() {
        let mut editor = Editor::default();
        for c in "a界é".chars() {
            editor.key(KeyCode::Char(c));
        }
        editor.key(KeyCode::Left);
        editor.key(KeyCode::Backspace);
        assert_eq!(editor.text, "aé");
        editor.key(KeyCode::Delete);
        assert_eq!(editor.text, "a");
    }

    #[test]
    fn connect_has_no_argument_completion_and_rejects_arguments() {
        let mut app = App::new(GromnieConfig::default());
        app.editor.set("/connect ".into());
        assert!(app.suggestions().is_empty());
        submit(&mut app, "/connect play.coldeve.ac");
        assert!(app.operation.is_none());
        assert!(app.server.is_none());
        assert!(
            app.transcript
                .back()
                .unwrap()
                .contains("takes no arguments")
        );
        app.config.accounts.insert(
            "tester".into(),
            AccountConfig {
                username: "tester".into(),
                password: "secret".into(),
                character: None,
            },
        );
        submit(&mut app, "/account tester");
        submit(&mut app, "/connect");
        assert!(app.operation.is_none());
        assert_eq!(app.editor.text, "/server ");
        assert!(app.transcript.back().unwrap().contains("Use /server"));
    }

    #[test]
    fn guards_and_terminal_sanitization() {
        let mut app = App::new(GromnieConfig::default());
        submit(&mut app, "/connect");
        assert!(app.operation.is_none());
        assert!(app.transcript.back().unwrap().contains("/account"));
        submit(&mut app, "/say hello");
        assert!(app.transcript.back().unwrap().contains("/enter"));
        app.push("\x1b[31mhello\x07");
        assert!(!app.transcript.back().unwrap().contains('\x1b'));
        assert!(!app.transcript.back().unwrap().contains('\x07'));
        submit(&mut app, "/exit");
        assert!(!app.quit);
        assert_eq!(app.transcript.back().unwrap(), "Not in the game world.");
        app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(!app.quit);
        app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
    }

    #[test]
    fn ctrl_c_requires_confirmation_and_other_keys_cancel() {
        let mut app = App::new(GromnieConfig::default());
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        app.key(ctrl_c);
        assert!(!app.quit);
        assert!(app.transcript.back().unwrap().contains("Ctrl-C again"));
        app.key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(!app.quit_confirmation);
        assert_eq!(app.editor.text, "/");
        app.key(ctrl_c);
        assert!(!app.quit);
        app.key(KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Repeat,
        ));
        assert!(!app.quit);
        app.key(ctrl_c);
        assert!(app.quit);
    }

    #[tokio::test]
    async fn returning_to_character_select_prints_the_list() {
        let mut app = App::new(GromnieConfig::default());
        let (tx, rx) = broadcast::channel(8);
        app.events = Some(rx);
        tx.send(ClientEvent::State(ClientStateEvent::CharacterSelect))
            .unwrap();
        app.tick().await;
        assert_eq!(
            app.transcript.back().unwrap(),
            "No enterable characters on this account."
        );
        assert!(!app.quit);
    }

    #[tokio::test]
    async fn slow_operations_do_not_block_editing_or_disconnect() {
        let mut app = App::new(GromnieConfig::default());
        let operation = tokio::spawn(std::future::pending());
        let abort = operation.abort_handle();
        app.operation = Some(operation);
        app.busy = "connecting";
        app.key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert_eq!(app.suggestions().len(), COMMANDS.len());
        submit(&mut app, "/account new");
        assert!(app.prompt.is_none());
        assert!(app.transcript.back().unwrap().contains("/disconnect"));
        submit(&mut app, "/disconnect");
        tokio::task::yield_now().await;
        assert!(abort.is_finished());
        assert!(app.operation.is_none());
        assert!(app.client.is_none());
        assert!(!app.quit);
    }

    #[tokio::test]
    async fn operation_errors_are_shown_and_leave_editor_usable() {
        let mut app = App::new(GromnieConfig::default());
        app.operation = Some(tokio::spawn(async { Err(ApiError::ConnectTimeout) }));
        tokio::task::yield_now().await;
        app.tick().await;
        assert!(app.operation.is_none());
        assert!(app.transcript.back().unwrap().contains("timed out"));
        submit(&mut app, "/account");
        assert!(matches!(app.prompt, Some(Prompt::Username)));
    }

    #[test]
    fn server_completion_and_suggestion_selection() {
        let mut config = GromnieConfig::default();
        config.servers.insert(
            "test".into(),
            gromnie_client::config::ServerConfig {
                host: "localhost".into(),
                port: 9000,
            },
        );
        let mut app = App::new(config);
        app.editor.set("/server t".into());
        assert_eq!(
            app.suggestions(),
            vec![("/server test".into(), "localhost:9000".into())]
        );
        app.complete();
        assert_eq!(app.editor.text, "/server test");
        app.editor.set("/".into());
        app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        app.complete();
        assert_eq!(app.editor.text, "/exit ");
    }

    #[test]
    fn welcome_disappears_on_first_output() {
        let mut app = App::new(GromnieConfig::default());
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content
            .chunks(100)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect();
        assert!(rows[0].starts_with("gromnie · no account · no server · disconnected"));
        assert!(rows[1].starts_with("gromnie — /account"));
        assert!(rows[2].starts_with("Tab completes"));
        assert!(!rows[3..].iter().any(|row| row.contains("Tab complete")));

        app.push("First output");
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.starts_with("First output"));
        assert!(!rendered.contains("gromnie — /account"));
        assert!(!rendered.contains("Tab complete"));
        assert_eq!(app.transcript.len(), 1);
    }

    #[test]
    fn status_uses_connection_state_and_character() {
        use gromnie_client::client::{CharacterSelectScene, ConnectingScene, InWorldScene};

        assert_eq!(status_details(None, false), ("offline", "—"));
        assert_eq!(status_details(None, true), ("connecting", "—"));
        let mut connecting = ConnectingScene::new();
        assert_eq!(
            status_details(Some(&Scene::Connecting(connecting.clone())), false),
            ("connecting", "—")
        );
        connecting.connect_progress = ConnectingProgress::ConnectResponseSent;
        assert_eq!(
            status_details(Some(&Scene::Connecting(connecting)), false),
            ("connected", "—")
        );
        assert_eq!(
            status_details(
                Some(&Scene::CharacterSelect(CharacterSelectScene::new(
                    "account".into(),
                    vec![]
                ))),
                false
            ),
            ("at char select", "—")
        );
        assert_eq!(
            status_details(
                Some(&Scene::InWorld(InWorldScene::new(1, "My Character".into()))),
                false
            ),
            ("in world", "My Character")
        );
    }

    #[test]
    fn status_is_pinned_only_at_the_bottom() {
        let mut app = App::new(GromnieConfig::default());
        for i in 0..40 {
            app.push(format!("Transcript line {i}"));
        }
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = buffer
            .content
            .chunks(100)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect();
        assert!(rows[0].starts_with("Transcript line"));
        assert!(!rows[..23].iter().any(|row| row.contains("offline")));
        let status = "offline · — · — · —";
        assert!(rows[23].ends_with(status));
        assert!(rows[23].trim_start() == status);
    }

    #[test]
    fn command_descriptions_are_column_aligned() {
        let mut app = App::new(GromnieConfig::default());
        app.editor.set("/".into());
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = buffer
            .content
            .chunks(100)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect();
        let columns: Vec<usize> = COMMANDS
            .iter()
            .map(|(_, description)| {
                let row = rows.iter().find(|row| row.contains(description)).unwrap();
                row[..row.find(description).unwrap()].width()
            })
            .collect();
        assert!(columns.iter().all(|column| *column == columns[0]));
        assert_eq!(columns[0], 2 + "/disconnect".width() + 2);
    }

    #[test]
    fn render_small_terminals_and_hide_passwords() {
        let mut app = App::new(GromnieConfig::default());
        app.prompt = Some(Prompt::Password("tester".into()));
        app.editor.set("never-print-this".into());
        for (width, height) in [(80, 24), (20, 8), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| app.draw(frame)).unwrap();
            let rendered: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(!rendered.contains("never-print-this"));
        }
    }
}
