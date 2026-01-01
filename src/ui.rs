use std::io;

use crossterm::cursor::{Hide, Show};

use crate::{
    mode::EditorMode,
    status_line::StatusLine,
    terminal::Terminal,
    ui_layout::{CursorPlacement, UiBody},
    ui_prompt_line::CommandLinePanel,
};
use vimrust_protocol::{CommandUiAccess, Frame, RpcRequest, StatusMessage, ViewportSink};

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
        let mut render = FrameRender::new(self, frame);
        frame.viewport().apply_to(&mut render);
        render.finish()
    }
}

pub enum UiQuitSignal {
    Requested,
    Idle,
}

struct FrameRender<'a, 'b> {
    ui: &'a mut Ui<'b>,
    frame: &'a Frame,
    result: io::Result<()>,
}

impl<'a, 'b> FrameRender<'a, 'b> {
    fn new(ui: &'a mut Ui<'b>, frame: &'a Frame) -> Self {
        Self {
            ui,
            frame,
            result: Ok(()),
        }
    }

    fn finish(self) -> io::Result<()> {
        self.result
    }

    fn render_with_size(&mut self, number_of_columns: u16, number_of_rows: u16) -> io::Result<()> {
        if number_of_rows == 0 {
            return Ok(());
        }

        let usable_rows = number_of_rows.saturating_sub(2);

        self.ui.terminal.clear_buffer();
        {
            self.ui.terminal.queue_add_command(Hide)?;

            let mut command_input = CommandInput::new();
            match self.frame.command_ui() {
                CommandUiAccess::Available(command_ui) => {
                    command_input.use_frame(&command_ui);
                }
                CommandUiAccess::Missing => {}
            }
            let mut command_line = command_input.panel(self.ui.terminal, number_of_columns);
            command_line.paint()?;

            if usable_rows > 0 {
                let body = UiBody::new(self.ui.mode, self.frame, number_of_columns, usable_rows);
                body.paint(self.ui.terminal)?;
            }

            if number_of_rows > 1 {
                self.ui.status_line.file_status_update(self.frame.status());
                self.ui.status_line.draw(
                    self.ui.terminal,
                    &self.ui.mode,
                    &self.frame.path(),
                    self.frame.position(),
                    number_of_columns,
                    number_of_rows,
                )?;
            }

            let cursor_placement =
                CursorPlacement::new(self.ui.mode, self.frame, number_of_columns, number_of_rows);
            let cursor = cursor_placement.command();
            self.ui.terminal.queue_add_command(cursor)?;
            self.ui.terminal.queue_add_command(Show)?;
        }

        self.ui.terminal.queue_execute()
    }
}

impl<'a, 'b> ViewportSink for FrameRender<'a, 'b> {
    fn size(&mut self, columns: u16, rows: u16) {
        self.result = self.render_with_size(columns, rows);
    }
}

struct CommandInput {
    text: String,
    selection: vimrust_protocol::CommandLineSelection,
}

impl CommandInput {
    fn new() -> Self {
        Self {
            text: String::new(),
            selection: vimrust_protocol::CommandLineSelection::None,
        }
    }

    fn use_frame(&mut self, command_ui: &vimrust_protocol::CommandUiFrame) {
        self.text = command_ui.command_text().to_string();
        self.selection = command_ui.command_selection();
    }

    fn panel<'a>(
        &self,
        terminal: &'a mut Terminal,
        number_of_columns: u16,
    ) -> CommandLinePanel<'a, '_> {
        CommandLinePanel::new(
            terminal,
            number_of_columns,
            &self.text,
            self.selection.clone(),
        )
    }
}
