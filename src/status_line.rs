use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{mode::EditorMode, terminal::Terminal};
use vimrust_protocol::{FilePath, StatusMessage};

/// Renders the status line on the last row of the screen.
pub struct StatusLine {
    message: StatusMessage,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            message: StatusMessage::Empty,
        }
    }

    pub fn message_clear(&mut self) {
        if !self.message.is_empty() {
            self.message = StatusMessage::Empty;
        }
    }

    pub fn message_update(&mut self, new_message: StatusMessage) {
        if self.message != new_message {
            self.message = new_message;
        }
    }

    pub fn draw(
        &self,
        terminal: &mut Terminal,
        mode: &EditorMode,
        file_path: &FilePath,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> io::Result<()> {
        // Leave one column of padding on both sides of the status line.
        let inner_width = number_of_columns.saturating_sub(2);
        let mut status = format!("{} > {}", mode.label(), file_path);
        if !self.message.is_empty() {
            status.push_str(" > ");
            self.message.append_to(&mut status);
        }
        if status.len() < inner_width as usize {
            status.push_str(&" ".repeat(inner_width as usize - status.len()));
        } else {
            status.truncate(inner_width as usize);
        }
        let status = format!(" {} ", status);
        terminal.queue_add_command(MoveTo(0, number_of_rows.saturating_sub(1)))?;
        terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
        terminal.queue_add_command(SetBackgroundColor(Color::Grey))?;
        terminal.queue_add_command(SetForegroundColor(Color::Black))?;
        terminal.queue_add_command(Print(status))?;
        terminal.queue_add_command(ResetColor)?;

        Ok(())
    }
}
