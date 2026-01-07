use crate::app::App;
use crate::components::{render_header, render_status_bar};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_debug_view(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    render_debug_content(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);
}

fn render_debug_content(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!("Messages ({})", app.network_messages.len()))
        .borders(Borders::ALL);

    let chat_widget = crate::widgets::chat::ChatWidget::new(&app.network_messages, |message| {
        let (color, prefix, opcode, description) = match message {
            crate::app::NetworkMessage::Sent {
                opcode,
                description,
                timestamp,
            } => (
                Color::Green,
                "→",
                opcode.clone(),
                format!("{} [{}]", description, timestamp.format("%H:%M:%S")),
            ),
            crate::app::NetworkMessage::Received {
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

        Line::from(vec![
            Span::styled(prefix, Style::default().fg(color).bold()),
            Span::raw(" "),
            Span::raw(opcode),
            Span::raw(" "),
            Span::raw(description),
        ])
    })
    .block(block);

    frame.render_widget(chat_widget, area);
}
