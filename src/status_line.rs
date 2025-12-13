use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{EditorMode, terminal::Terminal};

/// Renders the status line on the last row of the screen.
pub struct StatusLine;

impl StatusLine {
    pub fn draw(
        terminal: &mut Terminal,
        mode: &EditorMode,
        file_path: &Option<String>,
        status_message: &Option<String>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> io::Result<()> {
        let filename = file_path.as_deref().unwrap_or("[No Filename]");
        // Leave one column of padding on both sides of the status line.
        let inner_width = number_of_columns.saturating_sub(2);
        let mut status = format!("{} > {}", mode.label(), filename);
        if let Some(message) = status_message {
            if !message.is_empty() {
                status.push_str(" > ");
                status.push_str(message);
            }
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
