use std::fmt;

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatusMessage {
    Empty,
    Text { text: String },
}

impl StatusMessage {
    pub fn is_empty(&self) -> bool {
        matches!(self, StatusMessage::Empty)
    }

    pub fn append_to(&self, target: &mut String) {
        match self {
            StatusMessage::Empty => {}
            StatusMessage::Text { text } => target.push_str(text),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilePath {
    Missing,
    Provided { path: String },
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilePath::Missing => write!(f, "[No Filename]"),
            FilePath::Provided { path } => write!(f, "{}", path),
        }
    }
}

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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteKind {
    Backspace,
    Under,
}

#[derive(Deserialize, Serialize)]
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
    Command,
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
    pub kind: AckKind,
    pub message: StatusMessage,
    pub file_path: FilePath,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AckKind {
    Open,
    Save,
    SaveAs,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    pub mode: String,
    pub cursor: Cursor,
    pub rows: Vec<String>,
    pub status: StatusMessage,
    pub file_path: FilePath,
    pub size: (u16, u16),
    pub command_ui: Option<CommandUiFrame>,
    #[serde(default)]
    pub protocol_version: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandUiFrame {
    pub line: String,
    pub cursor_x: u16,
    pub focus_on_list: bool,
    pub list_items: Vec<CommandListItemFrame>,
    pub selected_index: Option<usize>,
    pub scroll_offset: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandListItemFrame {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CommandUiAction {
    StartPrompt,
    Clear,
    InsertChar { ch: char },
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    MoveSelectionUp,
    MoveSelectionDown,
    SelectFromList,
}
