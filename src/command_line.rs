use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;

/// Handles rendering of the command line (top row).
pub struct CommandLine {
    content: String,
    cursor_x: u16,
}

impl CommandLine {
    pub const PLACEHOLDER: &'static str = "Press : for commands";

    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_x: 0,
        }
    }

    pub fn start_prompt(&mut self) {
        self.content.clear();
        self.content.push(':');
        self.cursor_x = 1;
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_x = 0;
    }

    pub fn set_content(&mut self, new_content: String) {
        self.content = new_content;
        self.cursor_x = self.content.len() as u16;
    }

    pub fn command_line(&self) -> &str {
        self.content()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn command_cursor_x(&self) -> u16 {
        self.cursor_x()
    }

    pub fn cursor_x(&self) -> u16 {
        self.cursor_x
    }

    pub fn backspace(&mut self) {
        if self.cursor_x == 0 {
            return;
        }
        let delete_at = self.cursor_x.saturating_sub(1) as usize;
        if delete_at < self.content.len() {
            self.content.remove(delete_at);
            self.cursor_x = self.cursor_x.saturating_sub(1);
        }
    }

    pub fn delete(&mut self) {
        let delete_at = self.cursor_x as usize;
        if delete_at < self.content.len() {
            self.content.remove(delete_at);
            self.cursor_x = self.cursor_x.min(self.content.len() as u16);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x = self.cursor_x.saturating_sub(1);
        }
    }

    pub fn move_right(&mut self) {
        let limit = self.content.len() as u16;
        if self.cursor_x < limit {
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_x = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_x = self.content.len() as u16;
    }

    pub fn insert_char(&mut self, ch: char) {
        let insert_at = self.cursor_x as usize;
        if insert_at <= self.content.len() {
            self.content.insert(insert_at, ch);
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    pub fn draw(terminal: &mut Terminal, number_of_columns: u16, content: &str) -> io::Result<()> {
        terminal.queue_add_command(MoveTo(0, 0))?;
        terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
        let is_placeholder = content.is_empty();
        let display_content = if is_placeholder {
            Self::PLACEHOLDER
        } else {
            content
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
