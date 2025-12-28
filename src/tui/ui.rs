use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::prelude::*;
use std::io;

use crate::tui::app::App;
use crate::tui::views::{render_debug_view, render_game_view};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let options = ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Fullscreen,
        };
        let terminal = Terminal::with_options(backend, options)?;

        Ok(Self { terminal })
    }

    pub fn draw(&mut self, app: &App) -> io::Result<()> {
        self.terminal.draw(|frame| match app.current_view {
            crate::tui::app::AppView::Game => render_game_view(frame, app),
            crate::tui::app::AppView::Debug => render_debug_view(frame, app),
        })?;

        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        disable_raw_mode().ok();
    }
}

pub fn try_init_tui() -> io::Result<Tui> {
    Tui::new()
}
