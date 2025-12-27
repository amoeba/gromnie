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
    let status_text = if matches!(app.game_scene, GameScene::Logging { ddd_received: true }) {
        "Connected to server\n\nDDD Interrogation received ✓\n\nWaiting for character list..."
    } else {
        "Connecting to server...\n\nWaiting for DDD Interrogation..."
    };

    let paragraph = ratatui::widgets::Paragraph::new(status_text)
        .block(
            ratatui::widgets::Block::default()
                .title("Logging In")
                .borders(ratatui::widgets::Borders::ALL),
        )
        .alignment(ratatui::layout::Alignment::Center)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::White));

    frame.render_widget(paragraph, area);
}
