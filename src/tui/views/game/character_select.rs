use crate::tui::app::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_character_select_view(frame: &mut Frame, area: Rect, app: &App) {
    render_character_list(frame, area, app);
}

/// Render interactive character list with selection
fn render_character_list(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![];

    for (index, character) in app.client_status.characters.iter().enumerate() {
        let is_selected = index == app.selected_character_index;
        let delete_indicator = if character.delete_pending {
            " [PENDING DELETION]"
        } else {
            ""
        };

        let character_text = format!(
            "  {} (ID: {}){}",
            character.name, character.id, delete_indicator
        );

        if is_selected {
            // Highlight selected character
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Green).bold()),
                Span::styled(
                    character_text,
                    Style::default().bg(Color::DarkGray).fg(Color::White).bold(),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(character_text, Style::default().fg(Color::White)),
            ]));
        }
    }

    if app.client_status.characters.is_empty() {
        lines.push(Line::from("No characters available"));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" to select  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" to login"),
        ]));
    }

    let title = format!("{}: Character Select", app.client_status.account_name);
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}
