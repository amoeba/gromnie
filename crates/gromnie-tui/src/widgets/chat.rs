use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph, Widget},
};
use std::collections::VecDeque;

/// A chat widget that displays messages with proper scrolling to show the latest messages
pub struct ChatWidget<'a, T> {
    messages: &'a VecDeque<T>,
    block: Option<Block<'a>>,
    renderer: Box<dyn Fn(&T) -> Line<'static> + 'a>,
}

impl<'a, T> ChatWidget<'a, T> {
    pub fn new<F>(messages: &'a VecDeque<T>, renderer: F) -> Self
    where
        F: Fn(&T) -> Line<'static> + 'a,
    {
        Self {
            messages,
            block: None,
            renderer: Box::new(renderer),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a, T> Widget for ChatWidget<'a, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        let inner_area = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        let lines = self.messages_to_lines();
        let visible_lines = Self::visible_lines(lines, inner_area.height);

        let paragraph = Paragraph::new(visible_lines).style(Style::default().fg(Color::White));
        paragraph.render(inner_area, buf);
    }
}

impl<'a, T> ChatWidget<'a, T> {
    /// Convert chat messages to colored lines
    fn messages_to_lines(&self) -> Vec<Line<'static>> {
        self.messages.iter().map(|msg| (self.renderer)(msg)).collect()
    }

    /// Get only the lines that fit in the viewport, showing the bottom lines
    fn visible_lines(lines: Vec<Line<'static>>, height: u16) -> Vec<Line<'static>> {
        let height = height as usize;

        if lines.len() > height {
            let skip_count = lines.len() - height;
            lines.into_iter().skip(skip_count).collect()
        } else {
            lines
        }
    }
}
