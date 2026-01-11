/// Input handling and line editing

use crate::config::MAX_HISTORY_SIZE;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use arboard::Clipboard;

pub struct InputEditor {
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    saved_buffer: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum InputAction {
    None,
    Submit(String),
    Exit,
}

impl InputEditor {
    pub fn new() -> Self {
        InputEditor {
            buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            saved_buffer: None,
        }
    }

    /// Get the current input buffer
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Get the cursor position
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Clear the input buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        self.saved_buffer = None;
    }

    /// Add a command to history
    pub fn add_to_history(&mut self, command: String) {
        if command.is_empty() {
            return;
        }

        // Don't add duplicate consecutive entries
        if let Some(last) = self.history.last() {
            if last == &command {
                return;
            }
        }

        self.history.push(command);

        // Limit history size
        if self.history.len() > MAX_HISTORY_SIZE {
            self.history.remove(0);
        }
    }

    /// Handle a key event and return the action to take
    pub fn handle_key(&mut self, event: KeyEvent) -> InputAction {
        match event.code {
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                InputAction::Exit
            }
            KeyCode::Char('d') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                InputAction::Exit
            }
            KeyCode::Char('v') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                // Ctrl+V: Paste from clipboard
                self.paste_from_clipboard();
                InputAction::None
            }
            KeyCode::Char('V') if event.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
                // Ctrl+Shift+V: Paste from clipboard (common terminal shortcut)
                self.paste_from_clipboard();
                InputAction::None
            }
            KeyCode::Char(c) => {
                self.buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                InputAction::None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.buffer.remove(self.cursor_pos);
                }
                InputAction::None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.buffer.len() {
                    self.buffer.remove(self.cursor_pos);
                }
                InputAction::None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                InputAction::None
            }
            KeyCode::Right => {
                if self.cursor_pos < self.buffer.len() {
                    self.cursor_pos += 1;
                }
                InputAction::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                InputAction::None
            }
            KeyCode::End => {
                self.cursor_pos = self.buffer.len();
                InputAction::None
            }
            KeyCode::Up => {
                self.navigate_history_up();
                InputAction::None
            }
            KeyCode::Down => {
                self.navigate_history_down();
                InputAction::None
            }
            KeyCode::Enter => {
                let command = self.buffer.clone();
                InputAction::Submit(command)
            }
            _ => InputAction::None,
        }
    }

    /// Navigate up in command history
    fn navigate_history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Save current buffer and move to last history item
                self.saved_buffer = Some(self.buffer.clone());
                self.history_index = Some(self.history.len() - 1);
                self.buffer = self.history[self.history.len() - 1].clone();
                self.cursor_pos = self.buffer.len();
            }
            Some(idx) if idx > 0 => {
                // Move to previous history item
                self.history_index = Some(idx - 1);
                self.buffer = self.history[idx - 1].clone();
                self.cursor_pos = self.buffer.len();
            }
            _ => {}
        }
    }

    /// Navigate down in command history
    fn navigate_history_down(&mut self) {
        match self.history_index {
            Some(idx) if idx < self.history.len() - 1 => {
                // Move to next history item
                self.history_index = Some(idx + 1);
                self.buffer = self.history[idx + 1].clone();
                self.cursor_pos = self.buffer.len();
            }
            Some(_) => {
                // Restore saved buffer
                self.history_index = None;
                if let Some(saved) = self.saved_buffer.take() {
                    self.buffer = saved;
                    self.cursor_pos = self.buffer.len();
                }
            }
            None => {}
        }
    }

    /// Paste content from clipboard at cursor position
    fn paste_from_clipboard(&mut self) {
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                // Filter out newlines and control characters for single-line input
                let sanitized: String = text
                    .chars()
                    .filter(|c| !c.is_control() || *c == ' ')
                    .collect();

                // Insert at cursor position
                self.buffer.insert_str(self.cursor_pos, &sanitized);
                self.cursor_pos += sanitized.len();
            }
        }
    }
}

impl Default for InputEditor {
    fn default() -> Self {
        Self::new()
    }
}
