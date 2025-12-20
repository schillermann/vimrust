use std::io;

use crate::{
    EditorMode,
    command_ui_state::CommandUiState,
    editor::Editor,
    file::File,
    protocol::{Frame, RpcRequest},
    rpc::{build_frame, handle_request, RequestOutcome},
};

pub struct CoreState {
    editor: Editor,
    mode: EditorMode,
    status: Option<String>,
    size: (u16, u16),
    command_ui: CommandUiState,
}

impl CoreState {
    pub fn new(file_path: Option<String>) -> Self {
        let file = File::new(file_path);
        Self {
            editor: Editor::new(file),
            mode: EditorMode::Normal,
            status: None,
            size: (0, 0),
            command_ui: CommandUiState::new(),
        }
    }

    pub fn read_file(&mut self) -> io::Result<()> {
        self.editor.file_read()
    }

    pub fn set_size(&mut self, size: (u16, u16)) {
        self.size = size;
    }

    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    pub fn status(&self) -> &Option<String> {
        &self.status
    }

    pub fn handle(&mut self, request: RpcRequest) -> RequestOutcome {
        handle_request(
            request,
            &mut self.editor,
            &mut self.mode,
            &mut self.status,
            &mut self.size,
            &mut self.command_ui,
        )
    }

    pub fn frame(&self) -> Frame {
        let command_ui = if matches!(self.mode, EditorMode::Command) {
            Some(self.command_ui.frame())
        } else {
            None
        };
        build_frame(&self.editor, &self.mode, &self.status, self.size, command_ui)
    }
}
