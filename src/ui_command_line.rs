use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::CommandLineSelection;

pub(crate) struct CommandLinePanel<'a> {
    terminal: &'a mut Terminal,
    number_of_columns: u16,
    content: &'a str,
    selection: CommandLineSelection,
}

impl<'a> CommandLinePanel<'a> {
    pub(crate) fn new(
        terminal: &'a mut Terminal,
        number_of_columns: u16,
        content: &'a str,
        selection: CommandLineSelection,
    ) -> Self {
        Self {
            terminal,
            number_of_columns,
            content,
            selection,
        }
    }

    pub(crate) fn paint(&mut self) -> io::Result<()> {
        self.terminal.queue_add_command(MoveTo(0, 0))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        let placeholder = CommandLinePlaceholder::new();
        let display = placeholder.display_for(self.content);
        let display_content = display.text();

        // Leave one column of padding on both sides of the command line.
        let inner_width = self.number_of_columns.saturating_sub(2) as usize;
        let mut visible: String = display_content.chars().take(inner_width).collect();
        if visible.len() < inner_width {
            visible.push_str(&" ".repeat(inner_width - visible.len()));
        }
        let mut visible = format!(" {} ", visible);
        let target_width = self.number_of_columns as usize;
        if visible.len() < target_width {
            visible.push_str(&" ".repeat(target_width - visible.len()));
        } else if visible.len() > target_width {
            visible.truncate(target_width);
        }
        self.terminal.queue_add_command(SetBackgroundColor(Color::Rgb {
            r: 27,
            g: 27,
            b: 27,
        }))?;
        let highlight = CommandLineHighlight::new(self.selection.clone());
        let highlight_indices = highlight.visible_indices(display_content, inner_width);
        if highlight_indices.is_empty() {
            self.terminal
                .queue_add_command(SetForegroundColor(display.foreground()))?;
            self.terminal.queue_add_command(Print(visible))?;
        } else {
            CommandLineHighlight::queue(
                self.terminal,
                &visible,
                display.foreground(),
                highlight_indices,
            )?;
        }
        self.terminal.queue_add_command(ResetColor)?;
        Ok(())
    }
}

struct CommandLinePlaceholder {
    text: &'static str,
}

impl CommandLinePlaceholder {
    fn new() -> Self {
        Self {
            text: "Press : for commands",
        }
    }

    fn display_for(&self, content: &str) -> CommandLineDisplay {
        if content.is_empty() {
            CommandLineDisplay::placeholder(self.text)
        } else {
            CommandLineDisplay::content(content)
        }
    }
}

struct CommandLineDisplay {
    text: String,
    color: Color,
}

impl CommandLineDisplay {
    fn placeholder(text: &str) -> Self {
        Self {
            text: text.to_string(),
            color: Color::DarkGrey,
        }
    }

    fn content(text: &str) -> Self {
        Self {
            text: text.to_string(),
            color: Color::Grey,
        }
    }

    fn text(&self) -> &str {
        &self.text
    }

    fn foreground(&self) -> Color {
        self.color
    }
}

struct CommandLineHighlight {
    selection: CommandLineSelection,
}

impl CommandLineHighlight {
    fn new(selection: CommandLineSelection) -> Self {
        Self { selection }
    }

    fn visible_indices(&self, content: &str, inner_width: usize) -> Vec<usize> {
        let indices = self.selection.indices();
        if indices.is_empty() {
            return indices;
        }
        let visible_len = content.chars().take(inner_width).count();
        let mut visible = Vec::new();
        let mut idx = 0usize;
        while idx < indices.len() {
            let position = indices[idx];
            if position < visible_len {
                visible.push(position.saturating_add(1));
            }
            idx = idx.saturating_add(1);
        }
        visible
    }

    fn queue(
        terminal: &mut Terminal,
        line: &str,
        default_fg: Color,
        highlight_indices: Vec<usize>,
    ) -> io::Result<()> {
        let highlight_fg = Color::White;
        terminal.queue_add_command(SetForegroundColor(default_fg))?;
        let mut match_pos = 0usize;
        let mut next_match = if highlight_indices.is_empty() {
            usize::MAX
        } else {
            highlight_indices[0]
        };
        let mut idx = 0usize;
        for ch in line.chars() {
            if idx == next_match {
                terminal.queue_add_command(SetAttribute(Attribute::Italic))?;
                terminal.queue_add_command(SetForegroundColor(highlight_fg))?;
                terminal.queue_add_command(Print(ch))?;
                terminal.queue_add_command(SetAttribute(Attribute::Reset))?;
                terminal.queue_add_command(SetForegroundColor(default_fg))?;
                match_pos = match_pos.saturating_add(1);
                if match_pos < highlight_indices.len() {
                    next_match = highlight_indices[match_pos];
                } else {
                    next_match = usize::MAX;
                }
            } else {
                terminal.queue_add_command(Print(ch))?;
            }
            idx = idx.saturating_add(1);
        }
        Ok(())
    }
}
