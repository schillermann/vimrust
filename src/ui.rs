use std::io;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::KeyCode,
    style::Print,
    terminal::{Clear, ClearType},
};

use crate::{
    EditorMode, command_line::CommandLine, command_list::CommandList, editor::Editor,
    status_line::StatusLine, terminal::Terminal,
};

/// Responsible for orchestrating the per-frame UI rendering.
pub struct Ui<'a> {
    terminal: &'a mut Terminal,
    editor: &'a mut Editor,
    command_line: &'a mut CommandLine,
    command_list: &'a mut CommandList,
    status_line: StatusLine,
    updated: bool,
    command_focus_on_list: bool,
    quit: bool,
    mode: EditorMode,
}

impl<'a> Ui<'a> {
    pub fn new(
        terminal: &'a mut Terminal,
        editor: &'a mut Editor,
        command_line: &'a mut CommandLine,
        command_list: &'a mut CommandList,
    ) -> Self {
        Self {
            terminal,
            editor,
            command_line,
            command_list,
            status_line: StatusLine::new(),
            updated: false,
            command_focus_on_list: false,
            quit: false,
            mode: EditorMode::Normal,
        }
    }

    pub fn editor(&mut self) -> &mut Editor {
        self.updated = true;
        self.editor
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal.size()
    }

    pub fn editor_view_rows(&self) -> u16 {
        self.terminal.size().1.saturating_sub(2)
    }

    pub fn status_line(&mut self) -> &mut StatusLine {
        &mut self.status_line
    }

    pub fn set_status_message(&mut self, message: Option<String>) {
        self.status_line.message_update(message);
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

    pub fn mode_command_enter(&mut self) -> io::Result<()> {
        self.command_line.start_prompt();
        self.command_list.reset_selection();
        self.set_command_focus_on_list(false);
        self.enter_mode_command()
    }

    pub fn mode_command_exit(&mut self) -> io::Result<()> {
        self.set_mode(EditorMode::Normal)?;
        self.command_line.clear();
        self.command_list.reset_selection();
        self.set_command_focus_on_list(false);
        self.status_line.message_clear();
        Ok(())
    }

    pub fn command_list_enter_select(&mut self) {
        self.updated = true;
        let matches = self.command_list.filter(self.command_line.command_line());
        if self.command_focus_on_list && !matches.is_empty() {
            if let Some(selected) = self.command_list.command_selected_index() {
                let index = selected.min(matches.len() - 1);
                if let Some(entry) = matches.get(index) {
                    self.command_line.set_content(format!(":{}", entry.name));
                    self.set_command_focus_on_list(false);

                    let updated_matches =
                        self.command_list.filter(self.command_line.command_line());
                    if let Some(updated_index) = updated_matches
                        .iter()
                        .position(|candidate| candidate.name == entry.name)
                    {
                        self.command_list.set_selected_index(updated_index);
                        let list_rows = self.editor_view_rows().saturating_sub(3) as usize;
                        self.command_list.adjust_scroll_for_visible_rows(list_rows);
                    }
                }
            }
        }
    }

    pub fn command_enter(&mut self, file_path: &mut Option<String>) -> io::Result<()> {
        let was_focused_on_list = self.command_focus_on_list;
        self.command_list_enter_select();
        if self.command_focus_on_list {
            return Ok(());
        }
        if was_focused_on_list {
            // Just moved selection from list into the command line; wait for next Enter to execute.
            return Ok(());
        }

        let command = self
            .command_line
            .command_line()
            .trim_start_matches(':')
            .trim()
            .to_lowercase();

        match command.as_str() {
            "s" | "save" => {
                self.file_save(file_path);
            }
            "sq" => {
                self.file_save(file_path);
                self.quit = true;
                return Ok(());
            }
            "q" | "quit" => {
                self.quit = true;
                return Ok(());
            }
            _ => {}
        }

        self.set_command_focus_on_list(false);
        self.set_mode(EditorMode::Normal)?;
        Ok(())
    }

    pub fn command_line_backspace(&mut self) {
        self.updated = true;
        self.command_line.backspace();
        self.command_list.reset_selection();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_delete(&mut self) {
        self.updated = true;
        self.command_line.delete();
        self.command_list.reset_selection();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_move_left(&mut self) {
        self.updated = true;
        self.command_line.move_left();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_move_right(&mut self) {
        self.updated = true;
        self.command_line.move_right();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_move_home(&mut self) {
        self.updated = true;
        self.command_line.move_home();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_move_end(&mut self) {
        self.updated = true;
        self.command_line.move_end();
        self.set_command_focus_on_list(false);
    }

    pub fn command_line_insert_char(&mut self, ch: char) {
        self.updated = true;
        self.command_line.insert_char(ch);
        self.command_list.reset_selection();
        self.set_command_focus_on_list(false);
    }
    pub fn command_list_move_selection(&mut self, direction: KeyCode) {
        self.updated = true;
        let list_rows = self.editor_view_rows().saturating_sub(3) as usize;
        let matches = self.command_list.filter(self.command_line.command_line());
        if matches.is_empty() {
            self.command_list.reset_selection();
            self.set_command_focus_on_list(false);
            return;
        }

        self.set_command_focus_on_list(true);
        match self.command_list.command_selected_index() {
            None => match direction {
                KeyCode::Down => self.command_list.set_selected_index(0),
                KeyCode::Up => self
                    .command_list
                    .set_selected_index(matches.len().saturating_sub(1)),
                _ => {}
            },
            Some(current_index) => {
                let max_index = matches.len().saturating_sub(1);
                match direction {
                    KeyCode::Up if current_index > 0 => {
                        self.command_list
                            .set_selected_index(current_index.saturating_sub(1));
                    }
                    KeyCode::Down if current_index < max_index => {
                        self.command_list
                            .set_selected_index(current_index.saturating_add(1));
                    }
                    _ => {}
                }
            }
        }
        self.command_list.adjust_scroll_for_visible_rows(list_rows);
    }

    pub fn set_command_focus_on_list(&mut self, focus: bool) {
        self.command_focus_on_list = focus;
    }

    pub fn enter_mode_command(&mut self) -> io::Result<()> {
        self.updated = true;
        self.set_mode(EditorMode::Command)
    }

    pub fn set_mode(&mut self, mode: EditorMode) -> io::Result<()> {
        self.mode = mode;
        self.updated = true;
        self.terminal.set_cursor_style(&self.mode)
    }

    pub fn mode(&self) -> &EditorMode {
        &self.mode
    }

    pub fn file_save(&mut self, file_path: &mut Option<String>) {
        let new_message = match self.editor.file_save(file_path) {
            Ok(msg) => Some(msg),
            Err(err) => Some(format!("Error saving: {}", err)),
        };
        self.status_line.message_update(new_message);
        self.updated = true;
    }

    pub fn render(&mut self, file_path: &Option<String>) -> io::Result<()> {
        if !self.updated {
            return Ok(());
        }

        self.updated = false;

        let (number_of_columns, number_of_rows) = self.terminal_size();
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
                self.command_line.command_line(),
            )?;

            if usable_rows > 0 {
                if matches!(self.mode, EditorMode::Command) {
                    self.command_list.draw(
                        self.terminal,
                        number_of_columns,
                        1,
                        usable_rows,
                        self.command_line.command_line(),
                    )?;
                } else {
                    self.editor.scroll(number_of_columns, usable_rows);

                    let view = self.editor.view();
                    let rows = self
                        .editor
                        .rows_render(&view, number_of_columns, usable_rows);
                    for (idx, row) in rows.iter().enumerate() {
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
                    file_path,
                    number_of_columns,
                    number_of_rows,
                )?;
            }

            let (cursor_col, cursor_row) = match self.mode {
                EditorMode::Command
                    if self.command_focus_on_list
                        && self.command_list.command_selected_index().is_some() =>
                {
                    if let Some(selected_index) = self.command_list.command_selected_index() {
                        let relative_row = selected_index
                            .saturating_sub(self.command_list.command_scroll_offset())
                            as u16;
                        let list_row = 1u16
                            .saturating_add(1)
                            .saturating_add(2)
                            .saturating_add(relative_row)
                            .min(number_of_rows.saturating_sub(1));
                        (0, list_row)
                    } else {
                        (
                            self.command_line
                                .command_cursor_x()
                                .saturating_add(1)
                                .min(number_of_columns.saturating_sub(1)),
                            0,
                        )
                    }
                }
                EditorMode::Command => (
                    self.command_line
                        .command_cursor_x()
                        .saturating_add(1) // left padding on command line
                        .min(number_of_columns.saturating_sub(1)),
                    0,
                ),
                _ => {
                    let cursor_col = self
                        .editor
                        .cursor_x
                        .saturating_sub(self.editor.columns_offset)
                        .min(number_of_columns.saturating_sub(1));
                    let base_row = self
                        .editor
                        .cursor_y
                        .saturating_sub(self.editor.rows_offset)
                        .saturating_add(1);
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
}
