use std::io::{self, Stdout, Write};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    execute, queue,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size},
};

use crate::{buffer::Buffer, command_line::CommandLine, command_list::draw_command_list, editor::Editor, EditorMode};
use crate::status_line::StatusLine;

pub struct Terminal {
    size: (u16, u16),
    out: Stdout,
    buffer: Buffer,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = std::io::stdout();
        execute!(out, EnterAlternateScreen)?;
        let size = size()?;
        Ok(Self {
            size,
            out,
            buffer: Buffer::new(),
        })
    }

    pub fn cleanup(&mut self) {
        let _ = execute!(
            self.out,
            Clear(ClearType::All),
            MoveTo(0, 0),
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }

    pub fn update_size(&mut self) -> io::Result<()> {
        self.size = size()?;
        Ok(())
    }

    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    pub fn set_cursor_style(&mut self, mode: &EditorMode) -> io::Result<()> {
        let style = match mode {
            EditorMode::Normal => SetCursorStyle::DefaultUserShape,
            EditorMode::Edit => SetCursorStyle::SteadyBar,
            EditorMode::Command => SetCursorStyle::SteadyBar,
        };
        execute!(self.out, style)
    }

    pub fn add_command_to_queue<C: Command>(&mut self, command: C) -> io::Result<()> {
        let writer = self.buffer.writer();
        queue!(writer, command)?;
        Ok(())
    }

    pub fn render_frame(
        &mut self,
        editor: &mut Editor,
        mode: &EditorMode,
        file_path: &Option<String>,
        command_line: &str,
        command_cursor_x: u16,
        command_selected_index: usize,
        command_scroll_offset: usize,
        command_focus_on_list: bool,
    ) -> io::Result<()> {
        let (number_of_columns, number_of_rows) = self.size;
        if number_of_rows == 0 {
            return Ok(());
        }

        let usable_rows = number_of_rows.saturating_sub(2);

        self.buffer.clear();
        {
            self.add_command_to_queue(Hide)?;

            CommandLine::draw(self, number_of_columns, command_line)?;

            if usable_rows > 0 {
                if matches!(mode, EditorMode::Command) {
                    draw_command_list(
                        self,
                        number_of_columns,
                        1,
                        usable_rows,
                        command_line,
                        command_selected_index,
                        command_scroll_offset,
                    )?;
                } else {
                    editor.scroll(number_of_columns, usable_rows);

                    editor.draw_rows(self, number_of_columns, usable_rows, 1)?;
                }
            }

            if number_of_rows > 1 {
                StatusLine::draw(self, mode, file_path, number_of_columns, number_of_rows)?;
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
                    let cursor_col = editor
                        .cursor_x
                        .saturating_sub(editor.columns_offset)
                        .min(number_of_columns.saturating_sub(1));
                    let base_row = editor
                        .cursor_y
                        .saturating_sub(editor.rows_offset)
                        .saturating_add(1);
                    // Keep the edit cursor off the command-line row (row 0).
                    let min_editor_row = 1;
                    let max_editor_row = number_of_rows.saturating_sub(1).max(min_editor_row);
                    let cursor_row = base_row.max(1).min(max_editor_row);
                    (cursor_col, cursor_row)
                }
            };
            self.add_command_to_queue(MoveTo(cursor_col, cursor_row))?;
            self.add_command_to_queue(Show)?;
        }

        self.out.write_all(self.buffer.as_slice())?;
        self.out.flush()?;
        Ok(())
    }
}
