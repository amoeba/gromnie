use ratatui::Frame;
use std::error::Error;

use super::app::App;

pub trait Draw {
    fn draw(self, app: &mut App, frame: &mut Frame) -> Result<(), Box<dyn Error>>;
}
