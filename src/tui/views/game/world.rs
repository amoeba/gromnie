use crate::tui::app::{App, GameWorldState, GameWorldTab};
use ratatui::prelude::*;
use ratatui::widgets::*;

impl App {
    /// Switch to the next tab
    pub fn next_tab(&mut self) {
        self.game_world_tab = match self.game_world_tab {
            GameWorldTab::World => GameWorldTab::Chat,
            GameWorldTab::Chat => GameWorldTab::Map,
            GameWorldTab::Map => GameWorldTab::Inventory,
            GameWorldTab::Inventory => GameWorldTab::World,
        };
        // Don't auto-focus chat input when switching tabs
        // Chat input is only active when explicitly activated by Enter
    }

    /// Switch to the previous tab
    pub fn previous_tab(&mut self) {
        self.game_world_tab = match self.game_world_tab {
            GameWorldTab::World => GameWorldTab::Inventory,
            GameWorldTab::Chat => GameWorldTab::World,
            GameWorldTab::Map => GameWorldTab::Chat,
            GameWorldTab::Inventory => GameWorldTab::Map,
        };
        // Don't auto-focus chat input when switching tabs
        // Chat input is only active when explicitly activated by Enter
    }
}

pub fn render_game_world_view(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    state: &GameWorldState,
    created_objects: &[(u32, String)],
) {
    let title = format!("{}: Game World", app.client_status.account_name);

    let block = Block::default().title(title).borders(Borders::ALL);

    let inner = block.inner(area);

    frame.render_widget(block, area);

    // Create layout: tabs (top) | content (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Render scene tabs
    render_scene_tabs(frame, chunks[0], app);

    // Render tab content
    match app.game_world_tab {
        GameWorldTab::World => {
            render_world_tab(frame, chunks[1], app, state, created_objects);
        }
        GameWorldTab::Chat => {
            render_chat_tab(frame, chunks[1], app);
        }
        GameWorldTab::Map => {
            render_map_tab(frame, chunks[1]);
        }
        GameWorldTab::Inventory => {
            render_inventory_tab(frame, chunks[1]);
        }
    }
}

/// Render the scene tabs for GameWorld (World, Chat, Map, Inventory)
fn render_scene_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let tabs = ["World", "Chat", "Map", "Inventory"];
    let mut spans = vec![];

    for (idx, tab_name) in tabs.iter().enumerate() {
        let tab_enum = match idx {
            0 => GameWorldTab::World,
            1 => GameWorldTab::Chat,
            2 => GameWorldTab::Map,
            3 => GameWorldTab::Inventory,
            _ => unreachable!(),
        };

        if tab_enum == app.game_world_tab {
            // Active tab - highlight it
            spans.push(Span::styled(
                format!(" {} ", tab_name),
                Style::default().bg(Color::Blue).fg(Color::White).bold(),
            ));
        } else {
            // Inactive tab
            spans.push(Span::styled(
                format!(" {} ", tab_name),
                Style::default().fg(Color::Gray),
            ));
        }

        // Add separator between tabs
        if idx < tabs.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    let tab_bar = Paragraph::new(Line::from(spans)).style(Style::default().fg(Color::White));

    frame.render_widget(tab_bar, area);
}

/// Render the World tab
/// TODO: Implement world view rendering
fn render_world_tab(
    frame: &mut Frame,
    area: Rect,
    _app: &App,
    _state: &GameWorldState,
    _created_objects: &[(u32, String)],
) {
    let paragraph = Paragraph::new("TODO: World view")
        .block(Block::default().title("World").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}

/// Render the Chat tab
fn render_chat_tab(frame: &mut Frame, area: Rect, app: &App) {
    // Split into chat messages area and optional chat input area
    let chunks = if app.chat_input_active {
        // Show both messages and input when active
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area)
    } else {
        // Show only messages when input is not active
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    // Render chat messages
    let mut lines = vec![];
    if app.chat_messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "TODO: Chat messages",
            Style::default().fg(Color::Gray).italic(),
        )));
    } else {
        // Show most recent messages (up to height of area)
        let max_messages = (chunks[0].height as usize).saturating_sub(2); // Account for borders

        for message in app.chat_messages.iter().rev().take(max_messages).rev() {
            // Color based on message type
            let color = match message.message_type {
                0x00 => Color::White,   // Broadcast
                0x03 => Color::Cyan,    // Tell (incoming)
                0x04 => Color::Green,   // OutgoingTell
                0x05 => Color::Yellow,  // System
                0x06 => Color::Red,     // Combat
                0x07 => Color::Magenta, // Magic
                _ => Color::White,
            };

            lines.push(Line::from(Span::styled(
                message.text.clone(),
                Style::default().fg(color),
            )));
        }
    }

    let chat_messages_widget = Paragraph::new(lines)
        .block(Block::default().title("Messages").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(chat_messages_widget, chunks[0]);

    // Render chat input only when active
    if app.chat_input_active {
        let input_title = "Input (Enter to send, ESC to cancel)";

        let input_style = Style::default().fg(Color::Yellow);

        let chat_input_widget = Paragraph::new(app.chat_input.as_str())
            .block(
                Block::default()
                    .title(input_title)
                    .borders(Borders::ALL)
                    .border_style(input_style),
            )
            .style(input_style);

        frame.render_widget(chat_input_widget, chunks[1]);
    }
}

/// Render the Map tab
/// TODO: Implement map rendering
fn render_map_tab(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("TODO: Map view")
        .block(Block::default().title("Map").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}

/// Render the Inventory tab
/// TODO: Implement inventory rendering
fn render_inventory_tab(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("TODO: Inventory view")
        .block(Block::default().title("Inventory").borders(Borders::ALL))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}
