use std::io;

use crossterm::cursor::{Hide, MoveTo, Show};

use crate::{
    mode::EditorMode,
    status_line::StatusLine,
    terminal::Terminal,
    ui_prompt_line::CommandLinePanel,
    ui_layout::{CursorPlacement, UiBody},
};
use vimrust_protocol::{Frame, RpcRequest, StatusMessage};

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
            let mut command_line =
                CommandLinePanel::new(self.terminal, number_of_columns, command_text, command_selection);
            command_line.paint()?;

            if usable_rows > 0 {
                let body = UiBody::new(self.mode, frame, number_of_columns, usable_rows);
                body.paint(self.terminal)?;
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

            let cursor_placement =
                CursorPlacement::new(self.mode, frame, number_of_columns, number_of_rows);
            let (cursor_col, cursor_row) = cursor_placement.position();
            self.terminal
                .queue_add_command(MoveTo(cursor_col, cursor_row))?;
            self.terminal.queue_add_command(Show)?;
        }

        self.terminal.queue_execute()
    }

}

pub enum UiQuitSignal {
    Requested,
    Idle,
}
