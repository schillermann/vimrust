use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(transparent)]
pub struct ProtocolVersion {
    value: u32,
}

impl ProtocolVersion {
    pub fn current() -> Self {
        Self { value: 1 }
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::current()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatusMessage {
    Empty,
    Text { text: String },
}

impl StatusMessage {
    pub fn append_to(&self, target: &mut String) {
        match self {
            StatusMessage::Empty => {}
            StatusMessage::Text { text } => target.push_str(text),
        }
    }

    pub fn append_to_status_line(&self, target: &mut String) {
        match self {
            StatusMessage::Empty => {}
            StatusMessage::Text { text } => {
                target.push_str(" > ");
                target.push_str(text);
            }
        }
    }

    pub fn or(self, fallback: StatusMessage) -> StatusMessage {
        match self {
            StatusMessage::Empty => fallback,
            StatusMessage::Text { .. } => self,
        }
    }

    pub fn store(&mut self, message: StatusMessage) {
        *self = message;
    }

    pub fn clear(&mut self) {
        *self = StatusMessage::Empty;
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    mode: String,
    cursor: Cursor,
    rows: Vec<String>,
    status: StatusMessage,
    file_path: FilePath,
    size: (u16, u16),
    command_ui: Option<CommandUiFrame>,
    #[serde(default)]
    protocol_version: ProtocolVersion,
}

impl Frame {
    pub fn new(
        mode: String,
        cursor: Cursor,
        rows: Vec<String>,
        status: StatusMessage,
        file_path: FilePath,
        size: (u16, u16),
        command_ui: Option<CommandUiFrame>,
        protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            mode,
            cursor,
            rows,
            status,
            file_path,
            size,
            command_ui,
            protocol_version,
        }
    }

    pub fn mode_label(&self) -> &str {
        &self.mode
    }

    pub fn cursor_position(&self) -> Cursor {
        self.cursor.clone()
    }

    pub fn editor_rows(&self) -> &[String] {
        &self.rows
    }

    pub fn status_message(&self) -> StatusMessage {
        self.status.clone()
    }

    pub fn path(&self) -> FilePath {
        self.file_path.clone()
    }

    pub fn viewport(&self) -> (u16, u16) {
        self.size
    }

    pub fn command_ui_frame(&self) -> Option<&CommandUiFrame> {
        self.command_ui.as_ref()
    }

    pub fn version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn status_update(&mut self, status: StatusMessage) {
        self.status = status;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Cursor {
    col: u16,
    row: u16,
}

impl Cursor {
    pub fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }

    pub fn column_index(&self) -> u16 {
        self.col
    }

    pub fn row_index(&self) -> u16 {
        self.row
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandUiFrame {
    line: String,
    cursor_x: u16,
    focus_on_list: bool,
    list_items: Vec<CommandListItemFrame>,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl CommandUiFrame {
    pub fn new(
        line: String,
        cursor_x: u16,
        focus_on_list: bool,
        list_items: Vec<CommandListItemFrame>,
        selected_index: Option<usize>,
        scroll_offset: usize,
    ) -> Self {
        Self {
            line,
            cursor_x,
            focus_on_list,
            list_items,
            selected_index,
            scroll_offset,
        }
    }

    pub fn command_text(&self) -> &str {
        &self.line
    }

    pub fn cursor_column(&self) -> u16 {
        self.cursor_x
    }

    pub fn list_focus(&self) -> bool {
        self.focus_on_list
    }

    pub fn command_items(&self) -> &[CommandListItemFrame] {
        &self.list_items
    }

    pub fn selected_item(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn scroll_position(&self) -> usize {
        self.scroll_offset
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandListItemFrame {
    name: String,
    description: String,
}

impl CommandListItemFrame {
    pub fn new(name: String, description: String) -> Self {
        Self { name, description }
    }

    pub fn label(&self) -> &str {
        &self.name
    }

    pub fn detail(&self) -> &str {
        &self.description
    }
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
