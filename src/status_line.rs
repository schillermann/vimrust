use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{mode::EditorMode, terminal::Terminal};
use vimrust_protocol::{FilePath, StatusMessage, StatusPosition};

pub struct StatusLine {
    file_status: StatusMessage,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            file_status: StatusMessage::Empty,
        }
    }

    pub fn file_status_clear(&mut self) {
        self.file_status.clear();
    }

    pub fn file_status_update(&mut self, new_message: StatusMessage) {
        if self.file_status != new_message {
            self.file_status = new_message;
        }
    }

    pub fn draw(
        &self,
        terminal: &mut Terminal,
        mode: &EditorMode,
        file_path: &FilePath,
        position: StatusPosition,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> io::Result<()> {
        // Leave one column of padding on both sides of the status line.
        let inner_width = number_of_columns.saturating_sub(2);
        let mut status = String::new();
        mode.append_to(&mut status);
        status.push_str(" > ");
        status.push_str(&format!("{}", file_path));
        self.file_status.append_to_status_line(&mut status);
        let mut position_label = String::new();
        position.append_to(&mut position_label);
        let status_line = self.compose_line(&status, &position_label, inner_width as usize);
        let status = format!(" {} ", status_line);
        terminal.queue_add_command(MoveTo(0, number_of_rows.saturating_sub(2)))?;
        terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
        terminal.queue_add_command(SetBackgroundColor(Color::Grey))?;
        terminal.queue_add_command(SetForegroundColor(Color::Black))?;
        terminal.queue_add_command(Print(status))?;
        terminal.queue_add_command(ResetColor)?;

        Ok(())
    }

    fn compose_line(&self, left: &str, right: &str, inner_width: usize) -> String {
        if inner_width == 0 {
            return String::new();
        }

        let mut cells = vec![' '; inner_width];
        let mut left_chars: Vec<char> = left.chars().collect();
        if left_chars.len() > inner_width {
            left_chars.truncate(inner_width);
        }
        let mut idx = 0usize;
        while idx < left_chars.len() {
            cells[idx] = left_chars[idx];
            idx = idx.saturating_add(1);
        }

        let mut right_chars: Vec<char> = right.chars().collect();
        if right_chars.len() > inner_width {
            right_chars.truncate(inner_width);
        }
        let right_len = right_chars.len();
        if right_len > 0 {
            let start = inner_width.saturating_sub(right_len);
            let mut right_idx = 0usize;
            while right_idx < right_len {
                cells[start.saturating_add(right_idx)] = right_chars[right_idx];
                right_idx = right_idx.saturating_add(1);
            }
        }

        cells.into_iter().collect()
    }
}
