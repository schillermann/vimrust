use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};

use crate::{mode::EditorMode, terminal::Terminal};
use vimrust_protocol::CommandSelection;

pub(crate) struct CommandLinePanel<'a, 'b> {
    terminal: &'a mut Terminal,
    number_of_columns: u16,
    content: &'b str,
    selection: CommandSelection,
    focus: CommandLineFocus,
    mode: EditorMode,
}

impl<'a, 'b> CommandLinePanel<'a, 'b> {
    pub(crate) fn new(
        terminal: &'a mut Terminal,
        number_of_columns: u16,
        content: &'b str,
        selection: CommandSelection,
        focus: bool,
        mode: EditorMode,
    ) -> Self {
        Self {
            terminal,
            number_of_columns,
            content,
            selection,
            focus: CommandLineFocus { on_prompt: focus },
            mode,
        }
    }

    pub(crate) fn paint(&mut self) -> io::Result<()> {
        self.terminal.queue_add_command(MoveTo(0, 0))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        let placeholder = CommandLinePlaceholder {
            mode: self.mode,
            empty_text: "Press : for commands or ; for keymaps",
            visual_text: "Press : for visual commands or ; for keymaps",
            edit_text: "Press Esc to return to normal mode",
            command_text: ": type a command",
            keymap_text: "; filter keymaps",
        };
        let display = placeholder.display_for(self.content, &self.focus);
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
        self.terminal
            .queue_add_command(SetBackgroundColor(Color::Rgb {
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
    mode: EditorMode,
    empty_text: &'static str,
    visual_text: &'static str,
    edit_text: &'static str,
    command_text: &'static str,
    keymap_text: &'static str,
}

impl CommandLinePlaceholder {
    fn display_for(&self, content: &str, focus: &CommandLineFocus) -> CommandLineDisplay {
        if content.is_empty() {
            let message = match self.mode {
                EditorMode::Edit => self.edit_text,
                EditorMode::Visual => self.visual_text,
                _ => self.empty_text,
            };
            return CommandLineDisplay::placeholder(message, focus);
        }
        if content == ":" {
            return CommandLineDisplay::placeholder(self.command_text, focus);
        }
        if content == ";" {
            return CommandLineDisplay::placeholder(self.keymap_text, focus);
        }
        CommandLineDisplay::content(content, focus)
    }
}

struct CommandLineDisplay {
    text: String,
    color: Color,
}

impl CommandLineDisplay {
    fn placeholder(text: &str, focus: &CommandLineFocus) -> Self {
        Self {
            text: text.to_string(),
            color: focus.placeholder_foreground(),
        }
    }

    fn content(text: &str, focus: &CommandLineFocus) -> Self {
        Self {
            text: text.to_string(),
            color: focus.content_foreground(),
        }
    }

    fn text(&self) -> &str {
        &self.text
    }

    fn foreground(&self) -> Color {
        self.color
    }
}

struct CommandLineFocus {
    on_prompt: bool,
}

impl CommandLineFocus {
    fn placeholder_foreground(&self) -> Color {
        if self.on_prompt {
            Color::White
        } else {
            Color::DarkGrey
        }
    }

    fn content_foreground(&self) -> Color {
        if self.on_prompt {
            Color::White
        } else {
            Color::DarkGrey
        }
    }
}

struct CommandLineHighlight {
    selection: CommandSelection,
}

impl CommandLineHighlight {
    fn new(selection: CommandSelection) -> Self {
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
