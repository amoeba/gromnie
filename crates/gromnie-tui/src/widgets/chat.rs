use crate::app::ChatMessage;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use std::collections::VecDeque;

/// A chat widget that displays messages with proper scrolling to show the latest messages
pub struct ChatWidget<'a> {
    messages: &'a VecDeque<ChatMessage>,
    block: Option<Block<'a>>,
}

impl<'a> ChatWidget<'a> {
    pub fn new(messages: &'a VecDeque<ChatMessage>) -> Self {
        Self {
            messages,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl<'a> Widget for ChatWidget<'a> {
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

impl<'a> ChatWidget<'a> {
    /// Convert chat messages to colored lines
    fn messages_to_lines(&self) -> Vec<Line<'static>> {
        self.messages
            .iter()
            .map(|msg| {
                let color = match msg.message_type {
                    0x00 => Color::White,   // Broadcast
                    0x03 => Color::Cyan,    // Tell (incoming)
                    0x04 => Color::Green,   // OutgoingTell
                    0x05 => Color::Yellow,  // System
                    0x06 => Color::Red,     // Combat
                    0x07 => Color::Magenta, // Magic
                    _ => Color::White,
                };

                let text_span = Span::styled(msg.text.clone(), Style::default().fg(color));
                Line::from(text_span)
            })
            .collect()
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
