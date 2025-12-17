use std::io;

use crate::{
    EditorMode,
    editor::Editor,
    file::File,
    rpc::{build_frame, handle_request, CommandUiFrame, Frame, RequestOutcome, RpcRequest},
};

pub struct CoreState {
    editor: Editor,
    mode: EditorMode,
    status: Option<String>,
    size: (u16, u16),
}

impl CoreState {
    pub fn new(file_path: Option<String>) -> Self {
        let file = File::new(file_path);
        Self {
            editor: Editor::new(file),
            mode: EditorMode::Normal,
            status: None,
            size: (0, 0),
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
        )
    }

    pub fn frame(&self, command_ui: Option<CommandUiFrame>) -> Frame {
        build_frame(
            &self.editor,
            &self.mode,
            &self.status,
            self.size,
            command_ui,
        )
    }
}
