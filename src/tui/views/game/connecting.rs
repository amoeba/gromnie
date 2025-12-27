use crate::client::events::{CharacterInfo, ClientAction};
use crate::tui::app::{App, GameScene};

// Character selection functions that belong with the character selection view
impl App {
    /// Select the next character in the list
    pub fn select_next_character(&mut self) {
        if !self.client_status.characters.is_empty() {
            self.selected_character_index =
                (self.selected_character_index + 1) % self.client_status.characters.len();
        }
    }

    /// Select the previous character in the list
    pub fn select_previous_character(&mut self) {
        if !self.client_status.characters.is_empty() {
            if self.selected_character_index == 0 {
                self.selected_character_index = self.client_status.characters.len() - 1;
            } else {
                self.selected_character_index -= 1;
            }
        }
    }

    /// Get the currently selected character, if any
    pub fn get_selected_character(&self) -> Option<&CharacterInfo> {
        self.client_status
            .characters
            .get(self.selected_character_index)
    }

    /// Login with the selected character
    pub fn login_selected_character(&mut self) -> Result<(), String> {
        // Get character info first to avoid borrow conflicts
        let (character_id, character_name) = if let Some(character) = self.get_selected_character()
        {
            (character.id, character.name.clone())
        } else {
            return Err("No character selected".to_string());
        };

        if let Some(ref tx) = self.action_tx {
            // Immediately transition to GameWorld::LoggingIn
            self.game_scene = GameScene::GameWorld {
                state: crate::tui::app::GameWorldState::LoggingIn,
                created_objects: Vec::new(),
            };

            tx.send(ClientAction::LoginCharacter {
                character_id,
                character_name,
                account: self.client_status.account_name.clone(),
            })
            .map_err(|e| format!("Failed to send login action: {}", e))
        } else {
            Err("No action channel available".to_string())
        }
    }
}

pub fn render_connecting_view(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &App) {
    use ratatui::widgets::{Block, Borders, Paragraph, BorderType, Gauge};
    use ratatui::style::{Style, Color, Modifier};
    use ratatui::layout::{Layout, Constraint, Direction};

    // Create a block that fills the entire area
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title("Asheron's Call");

    frame.render_widget(outer_block.clone(), area);

    // Calculate inner area (excluding borders)
    let inner_area = outer_block.inner(area);

    // Split the inner area into two parts: main content and bottom status
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),  // Main content area
            Constraint::Length(3), // Bottom area for status bars
        ])
        .split(inner_area);

    // Main content area - centered "Asheron's Call" text
    let title_paragraph = Paragraph::new("Asheron's Call")
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

    frame.render_widget(title_paragraph, vertical_chunks[0]);

    // Bottom area - split for "Connecting" and "Updating" progress bars
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(vertical_chunks[1]);

    // "Connecting" progress bar (for authentication)
    // Use the fake progress value
    let connecting_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .title("Connecting")
        )
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(app.connecting_progress);

    frame.render_widget(connecting_gauge, bottom_chunks[0]);

    // "Updating" progress bar (for DDD messages)
    // Use the fake progress value
    let updating_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .title("Updating")
        )
        .gauge_style(Style::default().fg(Color::Yellow))
        .ratio(app.updating_progress);

    frame.render_widget(updating_gauge, bottom_chunks[1]);
}
