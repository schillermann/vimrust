use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{terminal::Terminal, EditorMode};

/// Renders the status line on the last row of the screen.
pub struct StatusLine;

impl StatusLine {
    pub fn draw(
        terminal: &mut Terminal,
        mode: &EditorMode,
        file_path: &Option<String>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> io::Result<()> {
        let filename = file_path.as_deref().unwrap_or("[No Filename]");
        // Leave one column of padding on both sides of the status line.
        let inner_width = number_of_columns.saturating_sub(2);
        let mut status = format!("{} > {}", mode.label(), filename);
        if status.len() < inner_width as usize {
            status.push_str(&" ".repeat(inner_width as usize - status.len()));
        } else {
            status.truncate(inner_width as usize);
        }
        let status = format!(" {} ", status);
        terminal.add_command_to_queue(MoveTo(0, number_of_rows.saturating_sub(1)))?;
        terminal.add_command_to_queue(Clear(ClearType::CurrentLine))?;
        terminal.add_command_to_queue(SetBackgroundColor(Color::Grey))?;
        terminal.add_command_to_queue(SetForegroundColor(Color::Black))?;
        terminal.add_command_to_queue(Print(status))?;
        terminal.add_command_to_queue(ResetColor)?;

        Ok(())
    }
}
