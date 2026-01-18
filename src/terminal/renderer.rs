/// Terminal rendering module

use crate::config::PROMPT;
use crossterm::{
    cursor, execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

pub struct TerminalRenderer {
    output_buffer: Vec<String>,
    last_printed_index: usize,
}

impl TerminalRenderer {
    pub fn new(welcome_message: String) -> Self {
        TerminalRenderer {
            output_buffer: vec![welcome_message],
            last_printed_index: 0,
        }
    }

    /// Add a line to the output buffer
    pub fn add_output(&mut self, line: String) {
        self.output_buffer.push(line);
    }

    /// Add multiple lines to the output buffer
    pub fn add_output_lines(&mut self, lines: Vec<String>) {
        for line in lines {
            self.output_buffer.push(line);
        }
    }

    /// Clear the output buffer
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }

    /// Render the terminal screen
    pub fn render(&self, input_buffer: &str, cursor_pos: usize) -> io::Result<()> {
        let mut stdout = io::stdout();

        // Clear screen
        execute!(stdout, terminal::Clear(ClearType::All))?;
        execute!(stdout, cursor::MoveTo(0, 0))?;

        // Get terminal size
        let (_, height) = terminal::size()?;

        // Calculate how many output lines to show
        let available_lines = (height as usize).saturating_sub(3);
        let start_idx = self.output_buffer.len().saturating_sub(available_lines);

        // Display output buffer
        for (i, line) in self.output_buffer.iter().skip(start_idx).enumerate() {
            queue!(stdout, cursor::MoveTo(0, i as u16))?;
            queue!(stdout, Print(line))?;
        }

        // Display separator
        let separator_line = height.saturating_sub(2);
        queue!(stdout, cursor::MoveTo(0, separator_line))?;
        queue!(stdout, SetForegroundColor(Color::DarkGrey))?;
        queue!(
            stdout,
            Print("─".repeat(terminal::size()?.0 as usize))
        )?;
        queue!(stdout, ResetColor)?;

        // Display input line
        let input_line = height.saturating_sub(1);
        queue!(stdout, cursor::MoveTo(0, input_line))?;
        queue!(stdout, SetForegroundColor(Color::Green))?;
        queue!(stdout, Print(PROMPT))?;
        queue!(stdout, ResetColor)?;
        queue!(stdout, Print(input_buffer))?;

        // Position cursor
        let prompt_len = PROMPT.len();
        queue!(
            stdout,
            cursor::MoveTo((prompt_len + cursor_pos) as u16, input_line)
        )?;

        stdout.flush()?;
        Ok(())
    }

    /// Clear the screen completely
    pub fn clear_screen() -> io::Result<()> {
        execute!(io::stdout(), terminal::Clear(ClearType::All))?;
        execute!(io::stdout(), cursor::MoveTo(0, 0))?;
        Ok(())
    }

    /// Get new output lines that haven't been printed yet (for non-interactive mode)
    pub fn get_new_output(&mut self) -> Vec<String> {
        let new_lines = self.output_buffer[self.last_printed_index..].to_vec();
        self.last_printed_index = self.output_buffer.len();
        new_lines
    }
}
