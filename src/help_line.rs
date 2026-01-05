use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::mode::EditorMode;
use crate::terminal::Terminal;

/// Renders the help line on the last row of the screen.
pub struct HelpLine {
    message: ModeHelpMessage,
}

impl HelpLine {
    pub fn new() -> Self {
        Self {
            message: ModeHelpMessage::new(),
        }
    }

    pub fn draw(
        &self,
        terminal: &mut Terminal,
        number_of_columns: u16,
        number_of_rows: u16,
        mode: &EditorMode,
    ) -> io::Result<()> {
        if number_of_rows == 0 {
            return Ok(());
        }

        let mut message = String::new();
        self.message.append_to(mode, &mut message);
        let line = HelpLineCells::new(&message, number_of_columns).cells;

        terminal.queue_add_command(MoveTo(0, number_of_rows.saturating_sub(1)))?;
        terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
        terminal.queue_add_command(SetAttribute(Attribute::Reset))?;
        terminal.queue_add_command(SetForegroundColor(Color::Grey))?;
        let mut idx = 0usize;
        let mut highlight = false;
        while idx < line.len() {
            let cell = &line[idx];
            if cell.highlight != highlight {
                highlight = cell.highlight;
                if highlight {
                    terminal.queue_add_command(SetAttribute(Attribute::Bold))?;
                    terminal.queue_add_command(SetForegroundColor(Color::White))?;
                } else {
                    terminal.queue_add_command(SetAttribute(Attribute::Reset))?;
                    terminal.queue_add_command(SetForegroundColor(Color::Grey))?;
                }
            }
            terminal.queue_add_command(Print(cell.ch))?;
            idx = idx.saturating_add(1);
        }
        terminal.queue_add_command(ResetColor)?;

        Ok(())
    }
}

struct ModeHelpMessage {
    normal: &'static str,
    edit: &'static str,
    visual: &'static str,
    prompt_command: &'static str,
    prompt_keymap: &'static str,
}

impl ModeHelpMessage {
    fn new() -> Self {
        Self {
            normal: "[h] [j] [k] [l] move  [e] edit  [v] select  [s] save  [q] quit",
            edit: "[Esc] normal  [Enter] newline  [Backspace]/[Delete] remove",
            visual: "[Esc] normal  [h] [j] [k] [l] move  [:] visual commands",
            prompt_command: "[Esc] normal  [Enter] execute  [Down] next command  [Up] previous command  [Ctrl+Down] list  [Ctrl+Up] input",
            prompt_keymap: "[Esc] normal  [Ctrl+Down] list  [Ctrl+Up] input",
        }
    }

    fn append_to(&self, mode: &EditorMode, target: &mut String) {
        match mode {
            EditorMode::Normal => target.push_str(self.normal),
            EditorMode::Edit => target.push_str(self.edit),
            EditorMode::Visual => target.push_str(self.visual),
            EditorMode::PromptCommand => target.push_str(self.prompt_command),
            EditorMode::PromptKeymap => target.push_str(self.prompt_keymap),
        }
    }
}

struct HelpLineCells {
    cells: Vec<HelpCell>,
}

impl HelpLineCells {
    fn new(message: &str, number_of_columns: u16) -> Self {
        let width = number_of_columns as usize;
        let mut cells = Vec::with_capacity(width);
        if width == 0 {
            return Self { cells };
        }

        let inner_width = width.saturating_sub(2);
        cells.push(HelpCell::new(' ', false));

        let mut inner = HelpLineCells::from_message(message, inner_width).cells;
        cells.append(&mut inner);

        if width > 1 {
            cells.push(HelpCell::new(' ', false));
        }

        while cells.len() < width {
            cells.push(HelpCell::new(' ', false));
        }
        if cells.len() > width {
            cells.truncate(width);
        }

        Self { cells }
    }

    fn from_message(message: &str, inner_width: usize) -> Self {
        let mut cells = Vec::with_capacity(inner_width);
        let mut parser = HelpLineParser::new(message);
        parser.append_to(&mut cells);
        if cells.len() > inner_width {
            cells.truncate(inner_width);
        }
        while cells.len() < inner_width {
            cells.push(HelpCell::new(' ', false));
        }
        Self { cells }
    }
}

struct HelpLineParser<'a> {
    chars: std::str::Chars<'a>,
    in_key: bool,
}

impl<'a> HelpLineParser<'a> {
    fn new(message: &'a str) -> Self {
        Self {
            chars: message.chars(),
            in_key: false,
        }
    }

    fn append_to(&mut self, cells: &mut Vec<HelpCell>) {
        while let Some(ch) = self.chars.next() {
            if ch == '[' {
                self.in_key = true;
                continue;
            }
            if ch == ']' {
                self.in_key = false;
                continue;
            }
            cells.push(HelpCell::new(ch, self.in_key));
        }
    }
}

struct HelpCell {
    ch: char,
    highlight: bool,
}

impl HelpCell {
    fn new(ch: char, highlight: bool) -> Self {
        Self { ch, highlight }
    }
}
