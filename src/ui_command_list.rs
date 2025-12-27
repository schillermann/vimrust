use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::CommandUiFrame;

pub(crate) struct CommandListPanel<'a> {
    terminal: &'a mut Terminal,
    cmd_ui: &'a CommandUiFrame,
    number_of_columns: u16,
    start_row: u16,
    number_of_rows: u16,
}

impl<'a> CommandListPanel<'a> {
    pub(crate) fn new(
        terminal: &'a mut Terminal,
        cmd_ui: &'a CommandUiFrame,
        number_of_columns: u16,
        start_row: u16,
        number_of_rows: u16,
    ) -> Self {
        Self {
            terminal,
            cmd_ui,
            number_of_columns,
            start_row,
            number_of_rows,
        }
    }

    pub(crate) fn paint(&mut self) -> io::Result<()> {
        if self.number_of_rows == 0 {
            return Ok(());
        }

        let matches = self.cmd_ui.command_items();
        let available_rows = self.number_of_rows.saturating_sub(3); // blank + header + divider
        let inner_width = self.number_of_columns.saturating_sub(2); // left/right padding
        let query = CommandQuery::new(self.cmd_ui.command_text());
        let mut name_width = 0;
        let mut idx = 0;
        while idx < matches.len() {
            let entry_len = matches[idx].label().len() as u16;
            if entry_len > name_width {
                name_width = entry_len;
            }
            idx += 1;
        }
        let name_width = name_width.min(inner_width);
        let command_col_width = name_width.max(6);
        let desc_col_width = inner_width
            .saturating_sub(command_col_width)
            .saturating_sub(1); // single space between columns

        let mut header = format!(
            "{:<cmd_width$}{}",
            "Command",
            if desc_col_width > 0 {
                format!(" {}", "Description")
            } else {
                String::new()
            },
            cmd_width = command_col_width as usize
        );
        if header.len() > inner_width as usize {
            header.truncate(inner_width as usize);
        } else {
            header.push_str(&" ".repeat(inner_width as usize - header.len()));
        }
        let header_line = format!(" {} ", header);
        self.terminal.queue_add_command(MoveTo(0, self.start_row))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(Print(format!(" {} ", " ".repeat(inner_width as usize))))?;
        self.terminal
            .queue_add_command(MoveTo(0, self.start_row.saturating_add(1)))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(SetAttribute(Attribute::Bold))?;
        self.terminal.queue_add_command(Print(header_line))?;
        self.terminal
            .queue_add_command(SetAttribute(Attribute::Reset))?;

        // Divider line under header
        self.terminal
            .queue_add_command(MoveTo(0, self.start_row.saturating_add(2)))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(Print(format!(" {} ", "─".repeat(inner_width as usize))))?;

        for row in 0..available_rows {
            let screen_row = self.start_row.saturating_add(row + 3);
            self.terminal.queue_add_command(MoveTo(0, screen_row))?;
            self.terminal
                .queue_add_command(Clear(ClearType::CurrentLine))?;

            if let Some(entry) =
                matches.get(self.cmd_ui.scroll_position().saturating_add(row as usize))
            {
                let is_selected = if let Some(selected_index) = self.cmd_ui.selected_item() {
                    selected_index == self.cmd_ui.scroll_position().saturating_add(row as usize)
                } else {
                    false
                };

                let mut name_display: String = entry
                    .label()
                    .chars()
                    .take(command_col_width as usize)
                    .collect();
                if name_display.len() < command_col_width as usize {
                    name_display
                        .push_str(&" ".repeat(command_col_width as usize - name_display.len()));
                }
                let mut desc_display = String::new();
                if desc_col_width > 0 {
                    desc_display = entry
                        .detail()
                        .chars()
                        .take(desc_col_width as usize)
                        .collect();
                    if desc_display.len() < desc_col_width as usize {
                        desc_display
                            .push_str(&" ".repeat(desc_col_width as usize - desc_display.len()));
                    }
                }

                let mut name_matches = Vec::new();
                let name_limit = name_display.chars().count();
                let name_indices = query.indices_for(entry.label());
                for index in name_indices {
                    if index < name_limit {
                        name_matches.push(index);
                    }
                }

                let mut desc_matches = Vec::new();
                if !desc_display.is_empty() {
                    let desc_limit = desc_display.chars().count();
                    let desc_indices = query.indices_for(entry.detail());
                    for index in desc_indices {
                        if index < desc_limit {
                            desc_matches.push(index);
                        }
                    }
                }

                if is_selected {
                    self.terminal.queue_add_command(Print(" "))?;
                    self.terminal
                        .queue_add_command(SetBackgroundColor(Color::DarkGrey))?;
                    self.terminal
                        .queue_add_command(SetForegroundColor(Color::White))?;
                    let name_highlight = CommandListHighlight::new(
                        &name_matches,
                        Some(Color::White),
                        Color::Yellow,
                        true,
                    );
                    name_highlight.paint(self.terminal, &name_display)?;
                    if !desc_display.is_empty() {
                        self.terminal.queue_add_command(Print(" "))?;
                        let desc_highlight = CommandListHighlight::new(
                            &desc_matches,
                            Some(Color::White),
                            Color::Yellow,
                            true,
                        );
                        desc_highlight.paint(self.terminal, &desc_display)?;
                    }
                    self.terminal.queue_add_command(ResetColor)?;
                    self.terminal.queue_add_command(Print(" "))?;
                } else {
                    self.terminal.queue_add_command(Print(" "))?;
                    let name_highlight =
                        CommandListHighlight::new(&name_matches, None, Color::Yellow, false);
                    name_highlight.paint(self.terminal, &name_display)?;
                    if !desc_display.is_empty() {
                        self.terminal.queue_add_command(Print(" "))?;
                        let desc_highlight =
                            CommandListHighlight::new(&desc_matches, None, Color::Yellow, false);
                        desc_highlight.paint(self.terminal, &desc_display)?;
                    }
                    self.terminal.queue_add_command(ResetColor)?;
                    self.terminal.queue_add_command(Print(" "))?;
                }
            }
        }

        Ok(())
    }
}

struct CommandQuery {
    normalized: String,
}

impl CommandQuery {
    fn new(command_line: &str) -> Self {
        let trimmed = command_line.trim_start_matches(':').trim();
        Self {
            normalized: trimmed.to_lowercase(),
        }
    }

    fn indices_for(&self, candidate: &str) -> Vec<usize> {
        if self.normalized.is_empty() {
            return Vec::new();
        }
        let mut positions = Vec::new();
        let mut q_iter = self.normalized.chars().peekable();
        let mut q_index = 0usize;
        let mut idx = 0usize;
        for ch in candidate.chars() {
            if let Some(&qch) = q_iter.peek() {
                if ch.eq_ignore_ascii_case(&qch) {
                    positions.push(idx);
                    q_iter.next();
                    q_index = q_index.saturating_add(1);
                    if q_index >= self.normalized.len() {
                        break;
                    }
                }
            } else {
                break;
            }
            idx = idx.saturating_add(1);
        }
        positions
    }
}

struct CommandListHighlight<'a> {
    match_indices: &'a [usize],
    default_fg: Option<Color>,
    highlight_fg: Color,
    keep_background: bool,
}

impl<'a> CommandListHighlight<'a> {
    fn new(
        match_indices: &'a [usize],
        default_fg: Option<Color>,
        highlight_fg: Color,
        keep_background: bool,
    ) -> Self {
        Self {
            match_indices,
            default_fg,
            highlight_fg,
            keep_background,
        }
    }

    fn paint(&self, terminal: &mut Terminal, text: &str) -> io::Result<()> {
        let mut match_pos = 0usize;
        let mut next_match = if self.match_indices.is_empty() {
            usize::MAX
        } else {
            self.match_indices[0]
        };

        if let Some(color) = self.default_fg {
            terminal.queue_add_command(SetForegroundColor(color))?;
        }

        let mut idx = 0usize;
        for ch in text.chars() {
            if idx == next_match {
                terminal.queue_add_command(SetForegroundColor(self.highlight_fg))?;
                terminal.queue_add_command(Print(ch))?;
                if let Some(color) = self.default_fg {
                    terminal.queue_add_command(SetForegroundColor(color))?;
                } else if !self.keep_background {
                    terminal.queue_add_command(ResetColor)?;
                }
                match_pos = match_pos.saturating_add(1);
                if match_pos < self.match_indices.len() {
                    next_match = self.match_indices[match_pos];
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
