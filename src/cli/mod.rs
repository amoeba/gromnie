pub mod app;
pub mod draw;
pub mod wizards;

pub use app::{App, AppScreen};
pub use draw::Draw;
pub use wizards::{ConfigWizard, LaunchWizard};

use ratatui::prelude::Backend;
use ratatui::{Frame, Terminal};
use std::error::Error;

pub fn draw(app: &mut App, frame: &mut Frame) {
    app.draw(frame);
}

pub fn run(app: &mut App, terminal: &mut Terminal<impl Backend>) -> Result<(), Box<dyn Error>> {
    app.run(terminal)
}
