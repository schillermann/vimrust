use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;

/// Handles rendering of the command line (top row).
pub struct CommandLine;

impl CommandLine {
    pub const PLACEHOLDER: &'static str = "Press : for commands";

    pub fn draw(
        terminal: &mut Terminal,
        number_of_columns: u16,
        command_line: &str,
    ) -> io::Result<()> {
        terminal.queue_add_command(MoveTo(0, 0))?;
        terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
        let is_placeholder = command_line.is_empty();
        let display_content = if is_placeholder {
            Self::PLACEHOLDER
        } else {
            command_line
        };

        // Leave one column of padding on both sides of the command line.
        let inner_width = number_of_columns.saturating_sub(2) as usize;
        let mut visible: String = display_content.chars().take(inner_width).collect();
        if visible.len() < inner_width {
            visible.push_str(&" ".repeat(inner_width - visible.len()));
        }
        let mut visible = format!(" {} ", visible);
        let target_width = number_of_columns as usize;
        if visible.len() < target_width {
            visible.push_str(&" ".repeat(target_width - visible.len()));
        } else if visible.len() > target_width {
            visible.truncate(target_width);
        }
        terminal.queue_add_command(SetBackgroundColor(Color::Rgb {
            r: 27,
            g: 27,
            b: 27,
        }))?;
        terminal.queue_add_command(SetForegroundColor(if is_placeholder {
            Color::DarkGrey
        } else {
            Color::Grey
        }))?;
        terminal.queue_add_command(Print(visible))?;
        terminal.queue_add_command(ResetColor)?;
        Ok(())
    }
}
