use std::io::{self, Stdout, Write};

use crossterm::{
    Command,
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

use crate::{
    EditorMode, buffer::Buffer, command_line::CommandLine, command_list::draw_command_list,
    editor_draw_rows, editor_scroll,
};

pub struct Terminal {
    pub size: (u16, u16),
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
        cursor_x: u16,
        cursor_y: u16,
        file_lines: &Vec<String>,
        mode: &EditorMode,
        file_path: &Option<String>,
        command_line: &str,
        command_cursor_x: u16,
        command_selected_index: usize,
        command_scroll_offset: usize,
        command_focus_on_list: bool,
        columns_offset: &mut u16,
        rows_offset: &mut u16,
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
                    editor_scroll(
                        cursor_x,
                        cursor_y,
                        number_of_columns,
                        usable_rows,
                        columns_offset,
                        rows_offset,
                    );

                    editor_draw_rows(
                        self,
                        number_of_columns,
                        usable_rows,
                        *columns_offset,
                        *rows_offset,
                        file_lines,
                        1,
                    )?;
                }
            }

            if number_of_rows > 1 {
                let filename = file_path.as_deref().unwrap_or("[No Filename]");
                // Leave one column of padding on both sides of the status line.
                let inner_width = number_of_columns.saturating_sub(2);
                let mut status = format!("{} > {}", mode.label(), filename);
                if status.len() < inner_width as usize {
                    status.push_str(&" ".repeat(inner_width as usize - status.len()));
                } else {
                    status.truncate(inner_width as usize);
                }
                let status = format!(" {} ", status);
                self.add_command_to_queue(MoveTo(0, number_of_rows.saturating_sub(1)))?;
                self.add_command_to_queue(Clear(ClearType::CurrentLine))?;
                self.add_command_to_queue(SetBackgroundColor(Color::Grey))?;
                self.add_command_to_queue(SetForegroundColor(Color::Black))?;
                self.add_command_to_queue(Print(status))?;
                self.add_command_to_queue(ResetColor)?;
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
                    let cursor_col = cursor_x
                        .saturating_sub(*columns_offset)
                        .min(number_of_columns.saturating_sub(1));
                    let base_row = cursor_y.saturating_sub(*rows_offset).saturating_add(1);
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
