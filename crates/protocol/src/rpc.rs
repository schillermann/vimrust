use serde::{Deserialize, Serialize};

use crate::{DocumentFile, Frame, PromptUiAction, StatusMessage};

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
        action: PromptUiAction,
    },
    ModeSet {
        mode: RequestEditorMode,
    },
    StateGet,
    EditorQuit,
    CommandExecute {
        #[serde(default)]
        line: CommandLine,
    },
}

#[derive(Clone)]
pub struct CommandLine {
    line: String,
    provided: bool,
}

impl CommandLine {
    pub fn provided(line: String) -> Self {
        Self {
            line,
            provided: true,
        }
    }

    pub fn from_ui() -> Self {
        Self {
            line: String::new(),
            provided: false,
        }
    }

    pub fn provided_line(&self) -> bool {
        self.provided
    }

    pub fn text(&self) -> &str {
        &self.line
    }
}

impl Default for CommandLine {
    fn default() -> Self {
        Self::from_ui()
    }
}

impl Serialize for CommandLine {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.provided {
            self.line.serialize(serializer)
        } else {
            serializer.serialize_none()
        }
    }
}

impl<'de> Deserialize<'de> for CommandLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let line = Option::<String>::deserialize(deserializer)?;
        Ok(match line {
            Some(line) => CommandLine::provided(line),
            None => CommandLine::from_ui(),
        })
    }
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
pub enum RequestEditorMode {
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
    file_path: DocumentFile,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AckKind {
    Open,
    Save,
    SaveAs,
}

impl Ack {
    pub fn new(kind: AckKind, message: StatusMessage, file_path: DocumentFile) -> Self {
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

    pub fn path(&self) -> DocumentFile {
        self.file_path.clone()
    }
}
