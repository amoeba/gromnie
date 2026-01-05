//! Keyboard event types for input handling
//!
//! This module provides platform-agnostic keyboard event types that abstract
//! from terminal-specific implementations like crossterm.

/// A keyboard event representing a key press, release, or repeat
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardEvent {
    /// The key that was pressed
    pub key: KeyCode,
    /// Modifier keys that were held during the event
    pub modifiers: KeyModifiers,
    /// The kind of keyboard event (press, release, or repeat)
    pub kind: KeyEventKind,
}

/// Represents a key on the keyboard
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCode {
    /// A character key (a-z, 0-9, symbols, etc.)
    Char(char),
    /// Enter/Return key
    Enter,
    /// Tab key
    Tab,
    /// Backspace key
    Backspace,
    /// Escape key
    Escape,
    /// Delete key
    Delete,
    /// Insert key
    Insert,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,
    /// Up arrow key
    Up,
    /// Down arrow key
    Down,
    /// Left arrow key
    Left,
    /// Right arrow key
    Right,
    /// Function keys F1-F12
    F(u8),
    /// Null/Unknown key
    Null,
}

/// Modifier keys that can be held during a keyboard event
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyModifiers {
    /// Control key is held
    pub ctrl: bool,
    /// Alt/Option key is held
    pub alt: bool,
    /// Shift key is held
    pub shift: bool,
}

/// The kind of keyboard event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    /// Key was pressed
    Press,
    /// Key was released
    Release,
    /// Key is being held (repeat event)
    Repeat,
}

impl KeyboardEvent {
    /// Create a new keyboard event
    pub fn new(key: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Self {
        Self {
            key,
            modifiers,
            kind,
        }
    }

    /// Create a keyboard event for a character press with no modifiers
    pub fn char_press(c: char) -> Self {
        Self::new(KeyCode::Char(c), KeyModifiers::default(), KeyEventKind::Press)
    }

    /// Create a keyboard event for a key press with no modifiers
    pub fn key_press(key: KeyCode) -> Self {
        Self::new(key, KeyModifiers::default(), KeyEventKind::Press)
    }
}

impl KeyModifiers {
    /// Create a new set of modifiers with all modifiers set to false
    pub fn new() -> Self {
        Self::default()
    }

    /// Create modifiers with ctrl set
    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            ..Default::default()
        }
    }

    /// Create modifiers with alt set
    pub fn alt() -> Self {
        Self {
            alt: true,
            ..Default::default()
        }
    }

    /// Create modifiers with shift set
    pub fn shift() -> Self {
        Self {
            shift: true,
            ..Default::default()
        }
    }

    /// Check if no modifiers are active
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift
    }
}
