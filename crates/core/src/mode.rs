use std::fs;

use vimrust_protocol::{FilePath, RequestEditorMode};

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorModeState {
    Normal,
    Edit,
    Visual,
    PromptCommand,
    PromptKeymap,
}

#[derive(Copy, Clone)]
pub(crate) struct EditorMode {
    current: EditorModeState,
}

impl EditorMode {
    pub fn new() -> Self {
        Self {
            current: EditorModeState::Normal,
        }
    }

    pub fn mode(&self) -> EditorModeState {
        self.current
    }

    pub fn prompt(&self) -> bool {
        matches!(
            self.current,
            EditorModeState::PromptCommand | EditorModeState::PromptKeymap
        )
    }

    pub fn prompt_command(&self) -> bool {
        matches!(self.current, EditorModeState::PromptCommand)
    }

    pub fn transition(&mut self, requested: RequestEditorMode, path: &FilePath) {
        self.current = match requested {
            RequestEditorMode::Normal => EditorModeState::Normal,
            RequestEditorMode::Edit => FileEditGate::new(self.current, path).mode(),
            RequestEditorMode::Visual => EditorModeState::Visual,
            RequestEditorMode::PromptCommand => EditorModeState::PromptCommand,
            RequestEditorMode::PromptKeymap => EditorModeState::PromptKeymap,
        };
    }
}

struct FileEditGate<'a> {
    current: EditorModeState,
    path: &'a FilePath,
}

impl<'a> FileEditGate<'a> {
    fn new(current: EditorModeState, path: &'a FilePath) -> Self {
        Self { current, path }
    }

    fn mode(&self) -> EditorModeState {
        match self.path {
            FilePath::Missing => EditorModeState::Edit,
            FilePath::Provided { path } => {
                let readonly = match fs::metadata(path) {
                    Ok(metadata) => metadata.permissions().readonly(),
                    Err(_) => false,
                };
                if readonly {
                    self.current
                } else {
                    EditorModeState::Edit
                }
            }
        }
    }
}
