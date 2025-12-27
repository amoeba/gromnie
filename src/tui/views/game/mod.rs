pub mod character_select;
pub mod connecting;
pub mod world;

// Re-export the game view rendering functions
pub use character_select::render_character_select_view;
pub use connecting::render_connecting_view;
pub use world::render_game_world_view;

use crate::tui::app::{App, GameScene};
use crate::tui::components::{render_header, render_status_bar};
use ratatui::prelude::*;

pub fn render_game_view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create main layout with header, content, and status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Header
    render_header(frame, chunks[0], app);

    // Main content - render appropriate scene
    match &app.game_scene {
        GameScene::Logging { ddd_received: _ } => {
            render_connecting_view(frame, chunks[1], app);
        }
        GameScene::CharacterSelect => {
            render_character_select_view(frame, chunks[1], app);
        }
        GameScene::GameWorld {
            state,
            created_objects,
        } => {
            render_game_world_view(frame, chunks[1], app, state, created_objects);
        }
    }

    // Status bar
    render_status_bar(frame, chunks[2], app);
}
