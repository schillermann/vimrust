use std::io;

use crossterm::cursor::{Hide, Show};

use crate::{
    file::UiFileSource,
    help_line::HelpLine,
    mode::EditorMode,
    status_line::StatusLine,
    terminal::Terminal,
    ui_layout::{CursorPlacement, UiBody},
    ui_prompt_line::PromptLine,
};
use vimrust_protocol::{CommandUiAccess, Frame, RpcRequest, StatusMessage, ViewportSink};

/// Responsible for orchestrating the per-frame UI rendering.
pub struct Ui<'a> {
    terminal: &'a mut Terminal,
    help_line: HelpLine,
    updated: bool,
    quit: bool,
    mode: EditorMode,
    file_status: StatusMessage,
}

impl<'a> Ui<'a> {
    pub fn new(terminal: &'a mut Terminal) -> Self {
        Self {
            terminal,
            help_line: HelpLine::new(),
            updated: false,
            quit: false,
            mode: EditorMode::Normal,
            file_status: StatusMessage::Empty,
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
        self.file_status_apply(message);
        self.updated = true;
    }

    pub fn status_clear(&mut self) {
        self.file_status.clear();
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
        let _ = self.terminal.apply_cursor_style(&self.mode);
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

    fn file_status_apply(&mut self, status: StatusMessage) {
        if self.file_status != status {
            self.file_status = status;
        }
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

        let usable_rows = number_of_rows.saturating_sub(3);

        self.ui.terminal.clear_buffer();
        {
            self.ui.terminal.queue_add_command(Hide)?;

            let command_input = match self.frame.command_ui() {
                CommandUiAccess::Available(command_ui) => PromptInput {
                    text: command_ui.command_text().to_string(),
                    selection: command_ui.command_selection(),
                    focus: command_ui.line_focus(),
                },
                CommandUiAccess::Missing => PromptInput::new(),
            };
            let mut command_line =
                command_input.panel(self.ui.terminal, number_of_columns, self.ui.mode);
            command_line.paint()?;

            if usable_rows > 0 {
                let body = UiBody::new(self.ui.mode, self.frame, number_of_columns, usable_rows);
                body.paint(self.ui.terminal)?;
            }

            if number_of_rows > 2 {
                self.ui.file_status_apply(self.frame.status());
                let status_line = StatusLine::new(
                    self.ui.terminal,
                    self.frame.path().file(),
                    self.ui.mode,
                    self.frame.position(),
                    self.ui.file_status.clone(),
                );
                status_line.draw(number_of_columns, number_of_rows)?;
            }

            if number_of_rows > 1 {
                self.ui.help_line.draw(
                    self.ui.terminal,
                    number_of_columns,
                    number_of_rows,
                    &self.ui.mode,
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

struct PromptInput {
    text: String,
    selection: vimrust_protocol::PromptInputSelection,
    focus: bool,
}

impl PromptInput {
    fn new() -> Self {
        Self {
            text: String::new(),
            selection: vimrust_protocol::PromptInputSelection::None,
            focus: false,
        }
    }

    fn panel<'a>(
        &self,
        terminal: &'a mut Terminal,
        number_of_columns: u16,
        mode: EditorMode,
    ) -> PromptLine<'a, '_> {
        PromptLine::new(
            terminal,
            number_of_columns,
            &self.text,
            self.selection.clone(),
            self.focus,
            mode,
        )
    }
}
