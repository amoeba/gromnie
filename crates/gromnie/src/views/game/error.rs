use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render_error_view(frame: &mut Frame, area: Rect, _app: &App, error_message: &str) {
    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create vertical layout with some padding
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(inner);

    // Error message
    let error_text = Paragraph::new(error_message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(error_text, chunks[1]);

    // Instructions at bottom
    let instructions = Paragraph::new("Press 'q' to quit or try logging in again")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).italic());
    frame.render_widget(instructions, chunks[2]);
}
