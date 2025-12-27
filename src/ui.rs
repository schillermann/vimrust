use std::io;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{Clear, ClearType},
};

use crate::{mode::EditorMode, status_line::StatusLine, terminal::Terminal};
use vimrust_protocol::{CommandUiFrame, Frame, RpcRequest, StatusMessage};

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

    pub fn resize_request(&self, suppress_frame: bool) -> RpcRequest {
        let size = self.terminal.size();
        RpcRequest::EditorResize {
            cols: size.0,
            rows: size.1,
            suppress_frame,
        }
    }

    pub fn status_update(&mut self, message: StatusMessage) {
        self.status_line.file_status_update(message);
        self.updated = true;
    }

    pub fn status_clear(&mut self) {
        self.status_line.file_status_clear();
        self.updated = true;
    }

    pub fn mark_dirty(&mut self) {
        self.updated = true;
    }

    pub fn quit_signal(&self) -> UiQuitSignal {
        if self.quit {
            UiQuitSignal::Requested
        } else {
            UiQuitSignal::Idle
        }
    }

    pub fn quit_request(&mut self) {
        self.quit = true;
        self.updated = true;
    }

    pub fn mode_apply(&mut self, mode: EditorMode) {
        self.mode = mode;
        self.updated = true;
        let _ = self.terminal.set_cursor_style(&self.mode);
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

        let (number_of_columns, number_of_rows) = frame.viewport();
        if number_of_rows == 0 {
            return Ok(());
        }

        let usable_rows = number_of_rows.saturating_sub(2);

        self.terminal.clear_buffer();
        {
            self.terminal.queue_add_command(Hide)?;

            let (command_text, command_selection) = match frame.command_ui_frame() {
                Some(command_ui) => (
                    command_ui.command_text(),
                    command_ui.command_selection(),
                ),
                None => ("", vimrust_protocol::CommandLineSelection::None),
            };
            draw_command_line(
                self.terminal,
                number_of_columns,
                command_text,
                command_selection,
            )?;

            if usable_rows > 0 {
                let body = UiModeBody {
                    mode: self.mode,
                    frame,
                    number_of_columns,
                    usable_rows,
                };
                body.draw(self)?;
            }

            if number_of_rows > 1 {
                self.status_line.file_status_update(frame.status_message());
                self.status_line.draw(
                    self.terminal,
                    &self.mode,
                    &frame.path(),
                    number_of_columns,
                    number_of_rows,
                )?;
            }

            let cursor_placement = CursorPlacement {
                mode: self.mode,
                frame,
                number_of_columns,
                number_of_rows,
            };
            let (cursor_col, cursor_row) = cursor_placement.position();
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

        let matches = cmd_ui.command_items();
        let available_rows = number_of_rows.saturating_sub(3); // blank + header + divider
        let inner_width = number_of_columns.saturating_sub(2); // left/right padding
        let query = Self::command_query_from_input(cmd_ui.command_text());
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

            if let Some(entry) = matches.get(cmd_ui.scroll_position().saturating_add(row as usize))
            {
                let is_selected = if let Some(selected_index) = cmd_ui.selected_item() {
                    selected_index == cmd_ui.scroll_position().saturating_add(row as usize)
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
                let name_indices = Self::matched_indices(&query, entry.label());
                for index in name_indices {
                    if index < name_limit {
                        name_matches.push(index);
                    }
                }

                let mut desc_matches = Vec::new();
                if !desc_display.is_empty() {
                    let desc_limit = desc_display.chars().count();
                    let desc_indices = Self::matched_indices(&query, entry.detail());
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

    fn draw_editor_rows(&mut self, frame: &Frame, usable_rows: u16) -> io::Result<()> {
        let rows = frame.editor_rows();
        let mut idx = 0usize;
        while idx < rows.len() {
            if idx as u16 >= usable_rows {
                break;
            }
            let row = &rows[idx];
            let screen_row = 1u16.saturating_add(idx as u16);
            self.terminal.queue_add_command(MoveTo(0, screen_row))?;
            self.terminal
                .queue_add_command(Clear(ClearType::CurrentLine))?;
            self.terminal.queue_add_command(Print(row))?;
            idx = idx.saturating_add(1);
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
        let mut idx = 0usize;
        for ch in candidate.chars() {
            if let Some(&qch) = q_iter.peek() {
                if ch.eq_ignore_ascii_case(&qch) {
                    positions.push(idx);
                    q_iter.next();
                    q_index = q_index.saturating_add(1);
                    if q_index >= query.len() {
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

    fn queue_highlighted(
        terminal: &mut Terminal,
        text: &str,
        match_indices: &[usize],
        default_fg: Option<Color>,
        highlight_fg: Color,
        keep_background: bool,
    ) -> io::Result<()> {
        let mut match_pos = 0usize;
        let mut next_match = if match_indices.is_empty() {
            usize::MAX
        } else {
            match_indices[0]
        };

        if let Some(color) = default_fg {
            terminal.queue_add_command(SetForegroundColor(color))?;
        }

        let mut idx = 0usize;
        for ch in text.chars() {
            if idx == next_match {
                terminal.queue_add_command(SetForegroundColor(highlight_fg))?;
                terminal.queue_add_command(Print(ch))?;
                if let Some(color) = default_fg {
                    terminal.queue_add_command(SetForegroundColor(color))?;
                } else if !keep_background {
                    terminal.queue_add_command(ResetColor)?;
                }
                match_pos = match_pos.saturating_add(1);
                if match_pos < match_indices.len() {
                    next_match = match_indices[match_pos];
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

pub enum UiQuitSignal {
    Requested,
    Idle,
}

struct UiModeBody<'a> {
    mode: EditorMode,
    frame: &'a Frame,
    number_of_columns: u16,
    usable_rows: u16,
}

impl<'a> UiModeBody<'a> {
    fn draw(&self, ui: &mut Ui<'_>) -> io::Result<()> {
        if self.usable_rows == 0 {
            return Ok(());
        }

        match self.mode {
            EditorMode::Command => {
                if let Some(cmd_ui) = self.frame.command_ui_frame() {
                    ui.draw_command_list_from_frame(
                        cmd_ui,
                        self.number_of_columns,
                        1,
                        self.usable_rows,
                    )?;
                }
            }
            _ => {
                ui.draw_editor_rows(self.frame, self.usable_rows)?;
            }
        }

        Ok(())
    }
}

struct CursorPlacement<'a> {
    mode: EditorMode,
    frame: &'a Frame,
    number_of_columns: u16,
    number_of_rows: u16,
}

impl<'a> CursorPlacement<'a> {
    fn position(&self) -> (u16, u16) {
        match self.mode {
            EditorMode::Command => {
                if let Some(cmd_ui) = self.frame.command_ui_frame() {
                    let mut command_cursor = (
                        cmd_ui
                            .cursor_column()
                            .saturating_add(1)
                            .min(self.number_of_columns.saturating_sub(1)),
                        0,
                    );
                    if cmd_ui.list_focus() {
                        if let Some(selected) = cmd_ui.selected_item() {
                            let relative_row =
                                selected.saturating_sub(cmd_ui.scroll_position()) as u16;
                            let list_row = 1u16
                                .saturating_add(1)
                                .saturating_add(2)
                                .saturating_add(relative_row)
                                .min(self.number_of_rows.saturating_sub(1));
                            command_cursor = (0, list_row);
                        }
                    }
                    command_cursor
                } else {
                    (0, 0)
                }
            }
            _ => {
                let cursor = self.frame.cursor_position();
                let cursor_col = cursor
                    .column_index()
                    .min(self.number_of_columns.saturating_sub(1));
                let base_row = cursor.row_index();
                // Keep the edit cursor off the command-line row (row 0).
                let min_editor_row = 1;
                let max_editor_row = self.number_of_rows.saturating_sub(1).max(min_editor_row);
                let cursor_row = base_row.max(1).min(max_editor_row);
                (cursor_col, cursor_row)
            }
        }
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

fn draw_command_line(
    terminal: &mut Terminal,
    number_of_columns: u16,
    content: &str,
    selection: vimrust_protocol::CommandLineSelection,
) -> io::Result<()> {
    terminal.queue_add_command(MoveTo(0, 0))?;
    terminal.queue_add_command(Clear(ClearType::CurrentLine))?;
    let placeholder = CommandLinePlaceholder::new();
    let display = placeholder.display_for(content);
    let display_content = display.text();

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
    let highlight = CommandLineHighlight::new(selection);
    let highlight_indices = highlight.visible_indices(display_content, inner_width);
    if highlight_indices.is_empty() {
        terminal.queue_add_command(SetForegroundColor(display.foreground()))?;
        terminal.queue_add_command(Print(visible))?;
    } else {
        CommandLineHighlight::queue(
            terminal,
            &visible,
            display.foreground(),
            highlight_indices,
        )?;
    }
    terminal.queue_add_command(ResetColor)?;
    Ok(())
}

struct CommandLineHighlight {
    selection: vimrust_protocol::CommandLineSelection,
}

impl CommandLineHighlight {
    fn new(selection: vimrust_protocol::CommandLineSelection) -> Self {
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
