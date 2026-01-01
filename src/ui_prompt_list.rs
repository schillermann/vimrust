use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::{CommandUiFrame, PromptMode};

pub(crate) struct PromptListView<'a> {
    terminal: &'a mut Terminal,
    cmd_ui: &'a CommandUiFrame,
    number_of_columns: u16,
    start_row: u16,
    number_of_rows: u16,
}

impl<'a> PromptListView<'a> {
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
        let is_keymap = self.cmd_ui.command_text().starts_with(';');
        let mut name_width = 0;
        let mut mode_width = 0;
        let mut key_width = 0;
        let mut idx = 0;
        while idx < matches.len() {
            let entry = &matches[idx];
            if is_keymap {
                let mode_label = match entry.mode() {
                    PromptMode::Command => "COMMAND",
                    PromptMode::Normal => "NORMAL",
                    PromptMode::Edit => "EDIT",
                    PromptMode::Visual => "VISUAL",
                    PromptMode::PromptCommand => "PROMPT_COMMAND",
                    PromptMode::PromptKeymap => "PROMPT_KEYMAP",
                };
                let mode_len = mode_label.chars().count() as u16;
                let key_len = entry.label().chars().count() as u16;
                if mode_len > mode_width {
                    mode_width = mode_len;
                }
                if key_len > key_width {
                    key_width = key_len;
                }
            } else {
                let entry_len = entry.label().len() as u16;
                if entry_len > name_width {
                    name_width = entry_len;
                }
            }
            idx += 1;
        }
        let (command_col_width, mode_col_width, key_col_width, desc_col_width) =
            PromptListLayout::new(is_keymap, inner_width, name_width, mode_width, key_width)
                .columns();

        let mut header = if is_keymap {
            PromptListHeader::new_keymap(key_col_width, mode_col_width, desc_col_width).line()
        } else {
            PromptListHeader::new_command(command_col_width, desc_col_width).line()
        };
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

                if is_keymap {
                    let mode_label = match entry.mode() {
                        PromptMode::Command => "COMMAND",
                        PromptMode::Normal => "NORMAL",
                        PromptMode::Edit => "EDIT",
                        PromptMode::Visual => "VISUAL",
                        PromptMode::PromptCommand => "PROMPT_COMMAND",
                        PromptMode::PromptKeymap => "PROMPT_KEYMAP",
                    };
                    let key_display = ColumnDisplay::new(entry.label(), key_col_width).render();
                    let mode_display = ColumnDisplay::new(mode_label, mode_col_width).render();
                    let desc_display = ColumnDisplay::new(entry.detail(), desc_col_width).render();

                    let mut mode_matches = Vec::new();
                    let mut key_matches = Vec::new();
                    let mode_indices = query.indices_for(mode_label);
                    let key_indices = query.indices_for(entry.label());
                    let mut pos = 0;
                    while pos < mode_indices.len() {
                        mode_matches.push(mode_indices[pos]);
                        pos = pos.saturating_add(1);
                    }
                    let mut key_pos = 0;
                    while key_pos < key_indices.len() {
                        key_matches.push(key_indices[key_pos]);
                        key_pos = key_pos.saturating_add(1);
                    }

                    let mut desc_matches = Vec::new();
                    if !desc_display.is_empty() {
                        let desc_limit = desc_display.chars().count();
                        let desc_indices = query.indices_for(entry.detail());
                        let mut desc_idx = 0;
                        while desc_idx < desc_indices.len() {
                            let index = desc_indices[desc_idx];
                            if index < desc_limit {
                                desc_matches.push(index);
                            }
                            desc_idx = desc_idx.saturating_add(1);
                        }
                    }

                    if is_selected {
                        self.terminal.queue_add_command(Print(" "))?;
                        self.terminal
                            .queue_add_command(SetBackgroundColor(Color::DarkGrey))?;
                        self.terminal
                            .queue_add_command(SetForegroundColor(Color::White))?;
                        if key_col_width > 0 {
                            let key_highlight = PromptListHighlight::new(
                                &key_matches,
                                Some(Color::White),
                                Color::Yellow,
                                true,
                            );
                            key_highlight.paint(self.terminal, &key_display)?;
                        }
                        if mode_col_width > 0 {
                            self.terminal.queue_add_command(Print(" "))?;
                            let mode_highlight = PromptListHighlight::new(
                                &mode_matches,
                                Some(Color::White),
                                Color::Yellow,
                                true,
                            );
                            mode_highlight.paint(self.terminal, &mode_display)?;
                        }
                        if !desc_display.is_empty() {
                            self.terminal.queue_add_command(Print(" "))?;
                            let desc_highlight = PromptListHighlight::new(
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
                        if key_col_width > 0 {
                            let key_highlight =
                                PromptListHighlight::new(&key_matches, None, Color::Yellow, false);
                            key_highlight.paint(self.terminal, &key_display)?;
                        }
                        if mode_col_width > 0 {
                            self.terminal.queue_add_command(Print(" "))?;
                            let mode_highlight =
                                PromptListHighlight::new(&mode_matches, None, Color::Yellow, false);
                            mode_highlight.paint(self.terminal, &mode_display)?;
                        }
                        if !desc_display.is_empty() {
                            self.terminal.queue_add_command(Print(" "))?;
                            let desc_highlight =
                                PromptListHighlight::new(&desc_matches, None, Color::Yellow, false);
                            desc_highlight.paint(self.terminal, &desc_display)?;
                        }
                        self.terminal.queue_add_command(ResetColor)?;
                        self.terminal.queue_add_command(Print(" "))?;
                    }
                } else {
                    let name_display =
                        ColumnDisplay::new(entry.label(), command_col_width).render();
                    let desc_display = ColumnDisplay::new(entry.detail(), desc_col_width).render();

                    let mut name_matches = Vec::new();
                    let name_limit = name_display.chars().count();
                    let name_indices = query.indices_for(entry.label());
                    let mut match_idx = 0;
                    while match_idx < name_indices.len() {
                        let index = name_indices[match_idx];
                        if index < name_limit {
                            name_matches.push(index);
                        }
                        match_idx = match_idx.saturating_add(1);
                    }

                    let mut desc_matches = Vec::new();
                    if !desc_display.is_empty() {
                        let desc_limit = desc_display.chars().count();
                        let desc_indices = query.indices_for(entry.detail());
                        let mut desc_idx = 0;
                        while desc_idx < desc_indices.len() {
                            let index = desc_indices[desc_idx];
                            if index < desc_limit {
                                desc_matches.push(index);
                            }
                            desc_idx = desc_idx.saturating_add(1);
                        }
                    }

                    if is_selected {
                        self.terminal.queue_add_command(Print(" "))?;
                        self.terminal
                            .queue_add_command(SetBackgroundColor(Color::DarkGrey))?;
                        self.terminal
                            .queue_add_command(SetForegroundColor(Color::White))?;
                        let name_highlight = PromptListHighlight::new(
                            &name_matches,
                            Some(Color::White),
                            Color::Yellow,
                            true,
                        );
                        name_highlight.paint(self.terminal, &name_display)?;
                        if !desc_display.is_empty() {
                            self.terminal.queue_add_command(Print(" "))?;
                            let desc_highlight = PromptListHighlight::new(
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
                            PromptListHighlight::new(&name_matches, None, Color::Yellow, false);
                        name_highlight.paint(self.terminal, &name_display)?;
                        if !desc_display.is_empty() {
                            self.terminal.queue_add_command(Print(" "))?;
                            let desc_highlight =
                                PromptListHighlight::new(&desc_matches, None, Color::Yellow, false);
                            desc_highlight.paint(self.terminal, &desc_display)?;
                        }
                        self.terminal.queue_add_command(ResetColor)?;
                        self.terminal.queue_add_command(Print(" "))?;
                    }
                }
            }
        }

        Ok(())
    }
}

struct PromptListLayout {
    is_keymap: bool,
    inner_width: u16,
    name_width: u16,
    mode_width: u16,
    key_width: u16,
}

impl PromptListLayout {
    fn new(
        is_keymap: bool,
        inner_width: u16,
        name_width: u16,
        mode_width: u16,
        key_width: u16,
    ) -> Self {
        Self {
            is_keymap,
            inner_width,
            name_width,
            mode_width,
            key_width,
        }
    }

    fn columns(&self) -> (u16, u16, u16, u16) {
        if self.is_keymap {
            let mut key_col_width = self.key_width.max(3).min(self.inner_width);
            let mut remaining = self.inner_width.saturating_sub(key_col_width);
            let mut mode_col_width = 0;
            let mut desc_col_width = 0;
            if remaining > 1 {
                mode_col_width = self.mode_width.max(4).min(remaining.saturating_sub(1));
                remaining = remaining.saturating_sub(mode_col_width);
                if mode_col_width > 0 && remaining > 1 {
                    desc_col_width = remaining.saturating_sub(1);
                }
            } else {
                key_col_width = self.inner_width;
            }
            (0, mode_col_width, key_col_width, desc_col_width)
        } else {
            let name_width = self.name_width.min(self.inner_width);
            let command_col_width = name_width.max(6);
            let desc_col_width = self
                .inner_width
                .saturating_sub(command_col_width)
                .saturating_sub(1);
            (command_col_width, 0, 0, desc_col_width)
        }
    }
}

struct PromptListHeader {
    line: String,
}

impl PromptListHeader {
    fn new_command(command_col_width: u16, desc_col_width: u16) -> Self {
        let line = format!(
            "{:<cmd_width$}{}",
            "Command",
            if desc_col_width > 0 {
                format!(" {}", "Description")
            } else {
                String::new()
            },
            cmd_width = command_col_width as usize
        );
        Self { line }
    }

    fn new_keymap(key_col_width: u16, mode_col_width: u16, desc_col_width: u16) -> Self {
        let mut line = format!("{:<key_width$}", "Key", key_width = key_col_width as usize);
        if mode_col_width > 0 {
            line.push(' ');
            line.push_str(&format!(
                "{:<mode_width$}",
                "Mode",
                mode_width = mode_col_width as usize
            ));
        }
        if desc_col_width > 0 {
            line.push(' ');
            line.push_str("Description");
        }
        Self { line }
    }

    fn line(&self) -> String {
        self.line.clone()
    }
}

struct ColumnDisplay<'a> {
    content: &'a str,
    width: u16,
}

impl<'a> ColumnDisplay<'a> {
    fn new(content: &'a str, width: u16) -> Self {
        Self { content, width }
    }

    fn render(&self) -> String {
        if self.width == 0 {
            return String::new();
        }
        let mut display: String = self.content.chars().take(self.width as usize).collect();
        if display.len() < self.width as usize {
            display.push_str(&" ".repeat(self.width as usize - display.len()));
        }
        display
    }
}

struct CommandQuery {
    normalized: String,
}

impl CommandQuery {
    fn new(command_line: &str) -> Self {
        let trimmed = command_line
            .trim_start_matches(':')
            .trim_start_matches(';')
            .trim();
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

struct PromptListHighlight<'a> {
    match_indices: &'a [usize],
    default_fg: Option<Color>,
    highlight_fg: Color,
    keep_background: bool,
}

impl<'a> PromptListHighlight<'a> {
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
