use std::io;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::KeyCode,
};

use crate::{
    EditorMode, command_line::CommandLine, command_list::draw_command_list, editor::Editor,
    status_line::StatusLine, terminal::Terminal,
};

/// Responsible for orchestrating the per-frame UI rendering.
pub struct Ui<'a> {
    terminal: &'a mut Terminal,
    editor: &'a mut Editor,
    updated: bool,
}

impl<'a> Ui<'a> {
    pub fn new(terminal: &'a mut Terminal, editor: &'a mut Editor) -> Self {
        Self {
            terminal,
            editor,
            updated: false,
        }
    }

    pub fn editor(&mut self) -> &mut Editor {
        self.updated = true;
        self.editor
    }

    pub fn terminal(&mut self) -> &mut Terminal {
        self.terminal
    }

    pub fn enter_mode_command(&mut self) -> io::Result<()> {
        self.updated = true;
        self.terminal.set_cursor_style(&EditorMode::Command)
    }

    pub fn enter_mode_edit(&mut self) -> io::Result<()> {
        self.updated = true;
        self.terminal.set_cursor_style(&EditorMode::Edit)?;
        self.editor.snap_cursor_to_tab_start();
        Ok(())
    }

    pub fn render(
        &mut self,
        mode: &EditorMode,
        file_path: &Option<String>,
        status_message: &Option<String>,
        command_line: &str,
        command_cursor_x: u16,
        command_selected_index: usize,
        command_scroll_offset: usize,
        command_focus_on_list: bool,
    ) -> io::Result<()> {
        if self.updated {
            self.updated = false;
            return Ok(());
        }

        let (number_of_columns, number_of_rows) = self.terminal.size();
        if number_of_rows == 0 {
            return Ok(());
        }

        let usable_rows = number_of_rows.saturating_sub(2);

        self.terminal.clear_buffer();
        {
            self.terminal.queue_add_command(Hide)?;

            CommandLine::draw(self.terminal, number_of_columns, command_line)?;

            if usable_rows > 0 {
                if matches!(mode, EditorMode::Command) {
                    draw_command_list(
                        self.terminal,
                        number_of_columns,
                        1,
                        usable_rows,
                        command_line,
                        command_selected_index,
                        command_scroll_offset,
                    )?;
                } else {
                    self.editor.scroll(number_of_columns, usable_rows);

                    self.editor
                        .draw_rows(self.terminal, number_of_columns, usable_rows, 1)?;
                }
            }

            if number_of_rows > 1 {
                StatusLine::draw(
                    self.terminal,
                    mode,
                    file_path,
                    status_message,
                    number_of_columns,
                    number_of_rows,
                )?;
            }

            let (cursor_col, cursor_row) = match mode {
                EditorMode::Command if command_focus_on_list => {
                    let relative_row =
                        command_selected_index.saturating_sub(command_scroll_offset) as u16;
                    let list_row = 1u16
                        .saturating_add(1)
                        .saturating_add(2)
                        .saturating_add(relative_row)
                        .min(number_of_rows.saturating_sub(1));
                    (0, list_row)
                }
                EditorMode::Command => (
                    command_cursor_x
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
