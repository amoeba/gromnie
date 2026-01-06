use crate::app::{App, GameWorldState, GameWorldTab};
use ratatui::prelude::*;
use ratatui::widgets::*;

impl App {
    /// Switch to the next tab
    pub fn next_tab(&mut self) {
        self.game_world_tab = match self.game_world_tab {
            GameWorldTab::World => GameWorldTab::Chat,
            GameWorldTab::Chat => GameWorldTab::Map,
            GameWorldTab::Map => GameWorldTab::Objects,
            GameWorldTab::Objects => GameWorldTab::World,
        };
        // Don't auto-focus chat input when switching tabs
        // Chat input is only active when explicitly activated by Enter
    }

    /// Switch to the previous tab
    pub fn previous_tab(&mut self) {
        self.game_world_tab = match self.game_world_tab {
            GameWorldTab::World => GameWorldTab::Objects,
            GameWorldTab::Chat => GameWorldTab::World,
            GameWorldTab::Map => GameWorldTab::Chat,
            GameWorldTab::Objects => GameWorldTab::Map,
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

    // Render based on game world state
    match state {
        GameWorldState::InPortalSpace => {
            render_portal_space(frame, inner);
        }
        GameWorldState::InWorld => {
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
                GameWorldTab::Objects => {
                    render_objects_tab(frame, chunks[1], app);
                }
            }
        }
    }
}

/// Render the scene tabs for GameWorld (World, Chat, Map, Objects)
fn render_scene_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let tabs = ["World", "Chat", "Map", "Objects"];
    let mut spans = vec![];

    for (idx, tab_name) in tabs.iter().enumerate() {
        let tab_enum = match idx {
            0 => GameWorldTab::World,
            1 => GameWorldTab::Chat,
            2 => GameWorldTab::Map,
            3 => GameWorldTab::Objects,
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

/// Render portal space - the animation loop while entering the game world
fn render_portal_space(frame: &mut Frame, area: Rect) {
    // Create a centered layout for the portal space message
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    // Portal space title
    let title = Paragraph::new("*** portal sounds ***")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).bold());

    frame.render_widget(title, chunks[0]);

    // Portal space message
    let message = Paragraph::new("Materializing into the world...")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(message, chunks[1]);

    // Loading indicator
    let loading = Paragraph::new("⏳")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(loading, chunks[2]);
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
/// Render the Objects tab - displays all tracked objects from ObjectTracker
fn render_objects_tab(frame: &mut Frame, area: Rect, app: &App) {
    use std::time::Duration;

    let object_count = app.object_tracker.object_count();

    // Collect and sort objects by last_updated (most recent first)
    let mut objects: Vec<_> = app.object_tracker.objects.values().collect();
    objects.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    // Define time thresholds for color coding
    let very_recent_threshold = Duration::from_secs(5);  // Green for last 5 seconds
    let recent_threshold = Duration::from_secs(30);       // Yellow for last 30 seconds

    // Create rows for each object
    let rows: Vec<Row> = objects
        .into_iter()
        .map(|obj| {
            let container_str = obj
                .container_id
                .map(|id| {
                    // Check if container is the player
                    if Some(id) == app.object_tracker.player_id {
                        // Look up player's name from tracker, fallback to account name
                        app.object_tracker
                            .get_object(id)
                            .map(|player| player.name.clone())
                            .unwrap_or_else(|| app.client_status.account_name.clone())
                    } else {
                        // Look up container object in the tracker to get its name
                        app.object_tracker
                            .get_object(id)
                            .map(|container| container.name.clone())
                            .unwrap_or_else(|| id.to_string())
                    }
                })
                .unwrap_or_else(|| "World".to_string());

            // Determine row color based on how recently the object was updated
            let time_since_update = obj.last_updated.elapsed();
            let row_color = if time_since_update < very_recent_threshold {
                Color::Green    // Very recent - bright green
            } else if time_since_update < recent_threshold {
                Color::Yellow   // Recent - yellow
            } else {
                Color::White    // Older - default white
            };

            Row::new(vec![
                obj.object_id.to_string(),
                obj.name.clone(),
                obj.object_type.clone(),
                container_str,
                obj.burden.to_string(),
            ]).style(Style::default().fg(row_color))
        })
        .collect();

    // Create table with header
    let header = Row::new(vec!["ObjectId", "Name", "Type", "Container", "Burden"])
        .style(Style::default().fg(Color::Yellow).bold());

    let table = Table::new(
        rows,
        [
            Constraint::Max(12),
            Constraint::Max(30),
            Constraint::Max(20),
            Constraint::Max(12),
            Constraint::Max(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!("Objects [Total: {}]", object_count))
            .borders(Borders::ALL),
    )
    .style(Style::default().fg(Color::White));

    frame.render_widget(table, area);
}
