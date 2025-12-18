use std::io;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};

use crate::{
    EditorMode,
    command_line::CommandLine,
    rpc::{CommandUiFrame, Frame},
    status_line::StatusLine,
    terminal::Terminal,
};

/// Responsible for orchestrating the per-frame UI rendering.
pub struct Ui<'a> {
    terminal: &'a mut Terminal,
    status_line: StatusLine,
    updated: bool,
    quit: bool,
    mode: EditorMode,
}

impl<'a> Ui<'a> {
    pub fn new(terminal: &'a mut Terminal) -> Self {
        Self {
            terminal,
            status_line: StatusLine::new(),
            updated: false,
            quit: false,
            mode: EditorMode::Normal,
        }
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal.size()
    }

    pub fn status_line(&mut self) -> &mut StatusLine {
        &mut self.status_line
    }

    pub fn set_status_message(&mut self, message: Option<String>) {
        self.status_line.file_message_update(message);
        self.updated = true;
    }

    pub fn quit(&self) -> bool {
        self.quit
    }

    pub fn set_quit(&mut self) {
        self.quit = true;
        self.updated = true;
    }

    pub fn set_mode_external(&mut self, mode: EditorMode) {
        self.mode = mode;
        self.updated = true;
        let _ = self.terminal.set_cursor_style(&self.mode);
    }

    pub fn mark_dirty(&mut self) {
        self.updated = true;
    }

    pub fn terminal_update_size(&mut self) -> io::Result<()> {
        self.terminal.size_update()?;
        self.updated = true;
        Ok(())
    }

    pub fn render_from_frame(&mut self, frame: &Frame) -> io::Result<()> {
        if !self.updated {
            return Ok(());
        }

        self.updated = false;

        let (number_of_columns, number_of_rows) = frame.size;
        if number_of_rows == 0 {
            return Ok(());
        }

        let usable_rows = number_of_rows.saturating_sub(2);

        self.terminal.clear_buffer();
        {
            self.terminal.queue_add_command(Hide)?;

            CommandLine::draw(
                self.terminal,
                number_of_columns,
                frame
                    .command_ui
                    .as_ref()
                    .map(|c| c.line.as_str())
                    .unwrap_or(""),
            )?;

            if usable_rows > 0 {
                if matches!(self.mode, EditorMode::Command) {
                    if let Some(cmd_ui) = &frame.command_ui {
                        self.draw_command_list_from_frame(
                            cmd_ui,
                            number_of_columns,
                            1,
                            usable_rows,
                        )?;
                    }
                } else {
                    for (idx, row) in frame.rows.iter().enumerate() {
                        if idx as u16 >= usable_rows {
                            break;
                        }
                        let screen_row = 1u16.saturating_add(idx as u16);
                        self.terminal.queue_add_command(MoveTo(0, screen_row))?;
                        self.terminal
                            .queue_add_command(Clear(ClearType::CurrentLine))?;
                        self.terminal.queue_add_command(Print(row))?;
                    }
                }
            }

            if number_of_rows > 1 {
                self.status_line.draw(
                    self.terminal,
                    &self.mode,
                    &frame.file_path,
                    number_of_columns,
                    number_of_rows,
                )?;
            }

            let (cursor_col, cursor_row) = match self.mode {
                EditorMode::Command => frame
                    .command_ui
                    .as_ref()
                    .map(|cmd_ui| {
                        if cmd_ui.focus_on_list
                            && let Some(selected) = cmd_ui.selected_index
                        {
                            let relative_row = selected.saturating_sub(cmd_ui.scroll_offset) as u16;
                            let list_row = 1u16
                                .saturating_add(1)
                                .saturating_add(2)
                                .saturating_add(relative_row)
                                .min(number_of_rows.saturating_sub(1));
                            return (0, list_row);
                        }
                        (
                            cmd_ui
                                .cursor_x
                                .saturating_add(1)
                                .min(number_of_columns.saturating_sub(1)),
                            0,
                        )
                    })
                    .unwrap_or((0, 0)),
                _ => {
                    let cursor_col = frame.cursor.col.min(number_of_columns.saturating_sub(1));
                    let base_row = frame.cursor.row;
                    // Keep the edit cursor off the command-line row (row 0).
                    let min_editor_row = 1;
                    let max_editor_row = number_of_rows.saturating_sub(1).max(min_editor_row);
                    let cursor_row = base_row.max(1).min(max_editor_row);
                    (cursor_col, cursor_row)
                }
            };
            self.terminal
                .queue_add_command(MoveTo(cursor_col, cursor_row))?;
            self.terminal.queue_add_command(Show)?;
        }

        self.terminal.queue_execute()
    }

    fn draw_command_list_from_frame(
        &mut self,
        cmd_ui: &CommandUiFrame,
        number_of_columns: u16,
        start_row: u16,
        number_of_rows: u16,
    ) -> io::Result<()> {
        if number_of_rows == 0 {
            return Ok(());
        }

        let matches = &cmd_ui.list_items;
        let available_rows = number_of_rows.saturating_sub(3); // blank + header + divider
        let inner_width = number_of_columns.saturating_sub(2); // left/right padding
        let query = Self::command_query_from_input(&cmd_ui.line);
        let name_width = matches
            .iter()
            .map(|c| c.name.len() as u16)
            .max()
            .unwrap_or(0)
            .min(inner_width);
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
        self.terminal.queue_add_command(MoveTo(0, start_row))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(Print(format!(" {} ", " ".repeat(inner_width as usize))))?;
        self.terminal
            .queue_add_command(MoveTo(0, start_row.saturating_add(1)))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(SetAttribute(Attribute::Bold))?;
        self.terminal.queue_add_command(Print(header_line))?;
        self.terminal
            .queue_add_command(SetAttribute(Attribute::Reset))?;

        // Divider line under header
        self.terminal
            .queue_add_command(MoveTo(0, start_row.saturating_add(2)))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        self.terminal
            .queue_add_command(Print(format!(" {} ", "─".repeat(inner_width as usize))))?;

        for row in 0..available_rows {
            let screen_row = start_row.saturating_add(row + 3);
            self.terminal.queue_add_command(MoveTo(0, screen_row))?;
            self.terminal
                .queue_add_command(Clear(ClearType::CurrentLine))?;

            if let Some(entry) = matches.get(cmd_ui.scroll_offset.saturating_add(row as usize)) {
                let is_selected = cmd_ui
                    .selected_index
                    .map(|idx| idx == cmd_ui.scroll_offset.saturating_add(row as usize))
                    .unwrap_or(false);

                let mut name_display: String = entry
                    .name
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
                        .description
                        .chars()
                        .take(desc_col_width as usize)
                        .collect();
                    if desc_display.len() < desc_col_width as usize {
                        desc_display
                            .push_str(&" ".repeat(desc_col_width as usize - desc_display.len()));
                    }
                }

                let name_matches: Vec<usize> = Self::matched_indices(&query, &entry.name)
                    .into_iter()
                    .filter(|idx| *idx < name_display.chars().count())
                    .collect();
                let desc_matches: Vec<usize> = if desc_display.is_empty() {
                    Vec::new()
                } else {
                    Self::matched_indices(&query, &entry.description)
                        .into_iter()
                        .filter(|idx| *idx < desc_display.chars().count())
                        .collect()
                };

                if is_selected {
                    self.terminal.queue_add_command(Print(" "))?;
                    self.terminal
                        .queue_add_command(SetBackgroundColor(Color::DarkGrey))?;
                    self.terminal
                        .queue_add_command(SetForegroundColor(Color::White))?;
                    Self::queue_highlighted(
                        self.terminal,
                        &name_display,
                        &name_matches,
                        Some(Color::White),
                        Color::Yellow,
                        true,
                    )?;
                    if !desc_display.is_empty() {
                        self.terminal.queue_add_command(Print(" "))?;
                        Self::queue_highlighted(
                            self.terminal,
                            &desc_display,
                            &desc_matches,
                            Some(Color::White),
                            Color::Yellow,
                            true,
                        )?;
                    }
                    self.terminal.queue_add_command(ResetColor)?;
                    self.terminal.queue_add_command(Print(" "))?;
                } else {
                    self.terminal.queue_add_command(Print(" "))?;
                    Self::queue_highlighted(
                        self.terminal,
                        &name_display,
                        &name_matches,
                        None,
                        Color::Yellow,
                        false,
                    )?;
                    if !desc_display.is_empty() {
                        self.terminal.queue_add_command(Print(" "))?;
                        Self::queue_highlighted(
                            self.terminal,
                            &desc_display,
                            &desc_matches,
                            None,
                            Color::Yellow,
                            false,
                        )?;
                    }
                    self.terminal.queue_add_command(ResetColor)?;
                    self.terminal.queue_add_command(Print(" "))?;
                }
            }
        }

        Ok(())
    }

    fn command_query_from_input(command_line: &str) -> String {
        let trimmed = command_line.trim_start_matches(':').trim();
        trimmed.to_lowercase()
    }

    fn matched_indices(query: &str, candidate: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut positions = Vec::new();
        let mut q_iter = query.chars().peekable();
        let mut q_index = 0usize;
        for (idx, ch) in candidate.chars().enumerate() {
            if let Some(&qch) = q_iter.peek() {
                if ch.eq_ignore_ascii_case(&qch) {
                    positions.push(idx);
                    q_iter.next();
                    q_index += 1;
                    if q_index >= query.len() {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        positions
    }

    fn queue_highlighted(
        terminal: &mut Terminal,
        text: &str,
        match_indices: &[usize],
        default_fg: Option<Color>,
        highlight_fg: Color,
        keep_background: bool,
    ) -> io::Result<()> {
        let mut match_iter = match_indices.iter().copied();
        let mut next_match = match_iter.next();

        if let Some(color) = default_fg {
            terminal.queue_add_command(SetForegroundColor(color))?;
        }

        for (idx, ch) in text.chars().enumerate() {
            if let Some(target) = next_match
                && idx == target
            {
                terminal.queue_add_command(SetForegroundColor(highlight_fg))?;
                terminal.queue_add_command(Print(ch))?;
                if let Some(color) = default_fg {
                    terminal.queue_add_command(SetForegroundColor(color))?;
                } else if !keep_background {
                    terminal.queue_add_command(ResetColor)?;
                }
                next_match = match_iter.next();
                continue;
            }
            terminal.queue_add_command(Print(ch))?;
        }

        Ok(())
    }
}
