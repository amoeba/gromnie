use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::app::{App, AppView, GameScene, GameWorldState, NetworkMessage};

pub fn render_game_view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Header
    render_header(frame, chunks[0], app);

    // Main content - render appropriate scene
    match &app.game_scene {
        GameScene::Logging { ddd_received } => {
            render_login_scene(frame, chunks[1], app, *ddd_received);
        }
        GameScene::CharacterSelect => {
            render_character_select_scene(frame, chunks[1], app);
        }
        GameScene::GameWorld { state, created_objects } => {
            render_game_world_scene(frame, chunks[1], app, state, created_objects);
        }
    }
}

pub fn render_debug_view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_header(frame, chunks[0], app);
    render_debug_content(frame, chunks[1], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        "Gromnie | {} | {} | {}",
        match app.current_view {
            AppView::Game => "Game View (TAB to switch)",
            AppView::Debug => "Debug View (TAB to switch)",
        },
        if app.client_status.connected {
            "Connected"
        } else {
            "Disconnected"
        },
        if app.client_status.logged_in {
            format!("Logged in: {}", app.client_status.current_character.as_deref().unwrap_or("Unknown"))
        } else {
            "Not logged in".to_string()
        }
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, area);
}

/// Scene 1: Login - shows connection status and DDD interrogation status
fn render_login_scene(frame: &mut Frame, area: Rect, app: &App, ddd_received: bool) {
    let status_text = if ddd_received {
        "Connected to server\n\nDDD Interrogation received ✓\n\nWaiting for character list..."
    } else {
        "Connecting to server...\n\nWaiting for DDD Interrogation..."
    };

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().title("Logging In").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Scene 2: Character Select - shows character list and allows selection
fn render_character_select_scene(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(area);

    // Account info
    let account_text = format!("Account: {}", app.client_status.account_name);
    let account_widget = Paragraph::new(account_text)
        .block(Block::default().title("Account").borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(account_widget, chunks[0]);

    // Character list
    render_character_list(frame, chunks[1], app);
}

/// Scene 3: Game World - shows created objects in the game world
fn render_game_world_scene(frame: &mut Frame, area: Rect, app: &App, state: &GameWorldState, created_objects: &[(u32, String)]) {
    // Create layout: objects panel (left) | chat window (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Render objects panel on the left
    render_objects_panel(frame, chunks[0], app, state, created_objects);

    // Render chat window on the right
    render_chat_panel(frame, chunks[1], app);
}

/// Render the objects panel showing game world objects
fn render_objects_panel(frame: &mut Frame, area: Rect, app: &App, state: &GameWorldState, created_objects: &[(u32, String)]) {
    let mut lines = vec![];

    // Add character info
    if let Some(ref character) = app.client_status.current_character {
        lines.push(Line::from(vec![
            Span::styled(
                format!("Character: {}", character),
                Style::default().fg(Color::Green).bold(),
            ),
        ]));
    }

    // Show state-specific status
    match state {
        GameWorldState::LoggingIn => {
            lines.push(Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    "Logging in...",
                    Style::default().fg(Color::Yellow).bold(),
                ),
            ]));
        }
        GameWorldState::LoggedIn => {
            lines.push(Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    "Logged in",
                    Style::default().fg(Color::Green).bold(),
                ),
            ]));
        }
        GameWorldState::LoggingOut => {
            lines.push(Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    "Logging out...",
                    Style::default().fg(Color::Red).bold(),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Add created objects list
    if created_objects.is_empty() {
        lines.push(Line::from(Span::styled(
            "Waiting for objects...",
            Style::default().fg(Color::Gray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("Objects ({})", created_objects.len()),
            Style::default().fg(Color::Yellow).bold(),
        )));
        lines.push(Line::from(""));

        for (object_id, object_name) in created_objects {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{} (ID: 0x{:08X})", object_name, object_id),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title("Game World").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render the chat panel showing chat messages and input
fn render_chat_panel(frame: &mut Frame, area: Rect, app: &App) {
    // Split into chat messages area and chat input area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Render chat messages
    let mut lines = vec![];
    if app.chat_messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "No messages yet. Press 'c' to start chatting.",
            Style::default().fg(Color::Gray).italic(),
        )));
    } else {
        // Show most recent messages (up to height of area)
        let max_messages = (chunks[0].height as usize).saturating_sub(2); // Account for borders
        for message in app.chat_messages.iter().rev().take(max_messages).rev() {
            // Color based on message type
            let color = match message.message_type {
                0x00 => Color::White,       // Broadcast
                0x03 => Color::Cyan,        // Tell (incoming)
                0x04 => Color::Green,       // OutgoingTell
                0x05 => Color::Yellow,      // System
                0x06 => Color::Red,         // Combat
                0x07 => Color::Magenta,     // Magic
                _ => Color::White,
            };

            lines.push(Line::from(Span::styled(
                message.text.clone(),
                Style::default().fg(color),
            )));
        }
    }

    let chat_messages_widget = Paragraph::new(lines)
        .block(Block::default().title("Chat").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(chat_messages_widget, chunks[0]);

    // Render chat input
    let input_title = if app.chat_input_focused {
        "Chat Input (ESC to cancel, Enter to send)"
    } else {
        "Chat Input (press 'c' to focus)"
    };

    let input_style = if app.chat_input_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let chat_input_widget = Paragraph::new(app.chat_input.as_str())
        .block(Block::default().title(input_title).borders(Borders::ALL).border_style(input_style))
        .style(input_style);

    frame.render_widget(chat_input_widget, chunks[1]);
}

#[allow(dead_code)]
fn render_status_panel(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = format!(
        "Account: {}\nConnected: {}\nLogged In: {}\nCurrent Character: {}",
        if app.client_status.account_name.is_empty() {
            "Unknown".to_string()
        } else {
            app.client_status.account_name.clone()
        },
        if app.client_status.connected { "Yes" } else { "No" },
        if app.client_status.logged_in { "Yes" } else { "No" },
        app.client_status
            .current_character
            .as_deref()
            .unwrap_or("None")
    );

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().title("Status").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

#[allow(dead_code)]
fn render_characters_panel(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![Line::from("Characters available:")];

    for character in &app.client_status.characters {
        let delete_indicator = if character.delete_pending {
            " [PENDING DELETION]"
        } else {
            ""
        };
        lines.push(Line::from(format!(
            "  {} (ID: {}){}",
            character.name, character.id, delete_indicator
        )));
    }

    if app.client_status.characters.is_empty() {
        lines.push(Line::from("  (none)"));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title("Characters").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render interactive character list with selection
fn render_character_list(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![];

    for (index, character) in app.client_status.characters.iter().enumerate() {
        let is_selected = index == app.selected_character_index;
        let delete_indicator = if character.delete_pending {
            " [PENDING DELETION]"
        } else {
            ""
        };

        let character_text = format!(
            "  {} (ID: {}){}",
            character.name, character.id, delete_indicator
        );

        if is_selected {
            // Highlight selected character
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    character_text,
                    Style::default().bg(Color::DarkGray).fg(Color::White).bold(),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(character_text, Style::default().fg(Color::White)),
            ]));
        }
    }

    if app.client_status.characters.is_empty() {
        lines.push(Line::from("No characters available"));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" to select  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" to login"),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title("Select Character").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

fn render_debug_content(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!("Messages ({})", app.network_messages.len()))
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // Show messages in reverse chronological order (most recent at top)
    for message in app.network_messages.iter().rev().take(area.height as usize - 2) {
        let (color, prefix, opcode, description) = match message {
            NetworkMessage::Sent {
                opcode,
                description,
                timestamp,
            } => (
                Color::Green,
                "→",
                opcode.clone(),
                format!("{} [{}]", description, timestamp.format("%H:%M:%S")),
            ),
            NetworkMessage::Received {
                opcode,
                description,
                timestamp,
            } => (
                Color::Yellow,
                "←",
                opcode.clone(),
                format!("{} [{}]", description, timestamp.format("%H:%M:%S")),
            ),
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(color).bold()),
            Span::raw(" "),
            Span::raw(opcode),
            Span::raw(" "),
            Span::raw(description),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}
