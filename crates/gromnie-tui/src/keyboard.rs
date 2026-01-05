//! Keyboard event conversion utilities
//!
//! Converts crossterm keyboard events to our platform-agnostic KeyboardEvent type.

use crossterm::event::{
    KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent,
    KeyEventKind as CrosstermKeyEventKind, KeyModifiers as CrosstermKeyModifiers,
};
use gromnie_events::{KeyCode, KeyEventKind, KeyModifiers, KeyboardEvent};

/// Convert a crossterm KeyEvent to our KeyboardEvent type
pub fn crossterm_to_keyboard_event(event: &CrosstermKeyEvent) -> KeyboardEvent {
    KeyboardEvent {
        key: convert_key_code(&event.code),
        modifiers: convert_modifiers(&event.modifiers),
        kind: convert_kind(&event.kind),
    }
}

/// Convert crossterm KeyCode to our KeyCode
fn convert_key_code(code: &CrosstermKeyCode) -> KeyCode {
    match code {
        CrosstermKeyCode::Char(c) => KeyCode::Char(*c),
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::Tab => KeyCode::Tab,
        CrosstermKeyCode::Backspace => KeyCode::Backspace,
        CrosstermKeyCode::Esc => KeyCode::Escape,
        CrosstermKeyCode::Delete => KeyCode::Delete,
        CrosstermKeyCode::Insert => KeyCode::Insert,
        CrosstermKeyCode::Home => KeyCode::Home,
        CrosstermKeyCode::End => KeyCode::End,
        CrosstermKeyCode::PageUp => KeyCode::PageUp,
        CrosstermKeyCode::PageDown => KeyCode::PageDown,
        CrosstermKeyCode::Up => KeyCode::Up,
        CrosstermKeyCode::Down => KeyCode::Down,
        CrosstermKeyCode::Left => KeyCode::Left,
        CrosstermKeyCode::Right => KeyCode::Right,
        CrosstermKeyCode::F(n) => KeyCode::F(*n),
        CrosstermKeyCode::BackTab => KeyCode::Tab, // BackTab is Shift+Tab
        CrosstermKeyCode::Null => KeyCode::Null,
        // Map other variants to Null for now
        _ => KeyCode::Null,
    }
}

/// Convert crossterm KeyModifiers to our KeyModifiers
fn convert_modifiers(modifiers: &CrosstermKeyModifiers) -> KeyModifiers {
    KeyModifiers {
        ctrl: modifiers.contains(CrosstermKeyModifiers::CONTROL),
        alt: modifiers.contains(CrosstermKeyModifiers::ALT),
        shift: modifiers.contains(CrosstermKeyModifiers::SHIFT),
    }
}

/// Convert crossterm KeyEventKind to our KeyEventKind
fn convert_kind(kind: &CrosstermKeyEventKind) -> KeyEventKind {
    match kind {
        CrosstermKeyEventKind::Press => KeyEventKind::Press,
        CrosstermKeyEventKind::Release => KeyEventKind::Release,
        CrosstermKeyEventKind::Repeat => KeyEventKind::Repeat,
    }
}
