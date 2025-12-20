use serde::{Deserialize, Serialize};

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
    pub message: Option<String>,
    pub file_path: Option<String>,
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
    pub status: Option<String>,
    pub file_path: Option<String>,
    pub size: (u16, u16),
    pub command_ui: Option<CommandUiFrame>,
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
