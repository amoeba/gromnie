use ratatui::prelude::*;
use ratatui::widgets::*;

/// Common header component
pub fn render_header(frame: &mut Frame, area: Rect, app: &crate::app::App) {
    // Split header area into two lines
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // First line: Title left-aligned, version right-aligned
    let version = env!("CARGO_PKG_VERSION");
    let title = "Gromnie";
    let version_text = format!("{{{}}}", version);

    let available_width =
        (chunks[0].width as usize).saturating_sub(title.len() + version_text.len() + 1);
    let spacing = " ".repeat(available_width.max(1));

    let spans = vec![
        Span::styled(" ", Style::default().bg(Color::Black).fg(Color::White)),
        Span::styled(title, Style::default().bg(Color::Black).fg(Color::White)),
        Span::styled(spacing, Style::default().bg(Color::Black).fg(Color::White)),
        Span::styled(
            version_text,
            Style::default().bg(Color::Black).fg(Color::White),
        ),
        Span::styled(" ", Style::default().bg(Color::Black).fg(Color::White)),
    ];

    let header_line = Line::from(spans);
    let paragraph =
        Paragraph::new(header_line).style(Style::default().bg(Color::Black).fg(Color::White));

    frame.render_widget(paragraph, chunks[0]);

    // Second line: Tab bar
    render_tab_bar(frame, chunks[1], app);
}

/// Common tab bar component
pub fn render_tab_bar(frame: &mut Frame, area: Rect, app: &crate::app::App) {
    let default_style = Style::default().bg(Color::Gray).fg(Color::Black);

    let game_style = if matches!(app.current_view, crate::app::AppView::Game) {
        Style::default().bg(Color::White).fg(Color::Black).bold()
    } else {
        default_style
    };

    let debug_style = if matches!(app.current_view, crate::app::AppView::Debug) {
        Style::default().bg(Color::White).fg(Color::Black).bold()
    } else {
        default_style
    };

    // Build the tab bar spans
    let mut spans = vec![
        Span::styled(" ", default_style),
        Span::styled("1 Game", game_style),
        Span::styled(" | ", default_style),
        Span::styled("2 Debug", debug_style),
    ];

    // Add world name if available
    if let Some(ref world_name) = app.client_status.world_name {
        spans.push(Span::styled(" | ", default_style));
        spans.push(Span::styled(
            format!("World: {}", world_name),
            Style::default().bg(Color::Gray).fg(Color::Black).bold(),
        ));
    }

    spans.push(Span::styled(" ", default_style));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(default_style);

    frame.render_widget(paragraph, area);
}

/// Common status bar component
pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &crate::app::App) {
    // Get connection status from the client status
    let connection_text = app.client_status.connection_status();

    // Get session and scene state display names (defer string creation until render time)
    let session_text = app.client_status.session_state.display_name();
    let scene_text = app.client_status.scene_state.display_name();

    // Determine style for connection state (bold if connected)
    let conn_style = if app.client_status.is_connected() {
        Style::default().bg(Color::White).fg(Color::Black).bold()
    } else {
        Style::default().bg(Color::White).fg(Color::Black)
    };

    // Determine style for scene state (bold if in world, dim if error)
    let scene_style = match &app.client_status.scene_state {
        crate::app::SceneState::InWorld => {
            Style::default().bg(Color::White).fg(Color::Black).bold()
        }
        crate::app::SceneState::Error(_) => Style::default().bg(Color::White).fg(Color::Red),
        _ => Style::default().bg(Color::White).fg(Color::Black),
    };

    // Create spans for each part
    let spans = vec![
        Span::styled(format!(" Conn: {}", connection_text), conn_style),
        Span::styled(" | ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::styled(
            format!("Session: {}", session_text),
            Style::default().bg(Color::White).fg(Color::Black),
        ),
        Span::styled(" | ", Style::default().bg(Color::White).fg(Color::Black)),
        Span::styled(format!("Scene: {}", scene_text), scene_style),
    ];

    let status_line = Line::from(spans);

    let paragraph =
        Paragraph::new(status_line).style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_widget(paragraph, area);
}
