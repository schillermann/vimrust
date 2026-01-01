use serde::{Deserialize, Serialize};

use crate::{CommandUiAction, FilePath, Frame, StatusMessage};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcRequest {
    EditorResize {
        cols: u16,
        rows: u16,
        suppress_frame: bool,
    },
    FileOpen {
        path: String,
    },
    FileSave,
    FileSaveAs {
        path: String,
    },
    TextInsert {
        text: String,
    },
    TextDelete {
        kind: DeleteKind,
    },
    LineBreak,
    CursorMove {
        direction: MoveDirection,
    },
    CommandUi {
        action: CommandUiAction,
    },
    ModeSet {
        mode: RpcMode,
    },
    StateGet,
    EditorQuit,
    CommandExecute {
        line: Option<String>,
    },
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DeleteKind {
    Backspace,
    Under,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MoveDirection {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMode {
    Normal,
    Edit,
    Visual,
    PromptCommand,
    PromptKeymap,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RpcResponse {
    Frame(Frame),
    Ack(Ack),
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Ack {
    kind: AckKind,
    message: StatusMessage,
    file_path: FilePath,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AckKind {
    Open,
    Save,
    SaveAs,
}

impl Ack {
    pub fn new(kind: AckKind, message: StatusMessage, file_path: FilePath) -> Self {
        Self {
            kind,
            message,
            file_path,
        }
    }

    pub fn kind(&self) -> AckKind {
        self.kind.clone()
    }

    pub fn message(&self) -> StatusMessage {
        self.message.clone()
    }

    pub fn path(&self) -> FilePath {
        self.file_path.clone()
    }
}
