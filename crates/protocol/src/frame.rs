use serde::{Deserialize, Serialize};

use crate::{CommandUiFrame, FilePath, ProtocolVersion, StatusMessage};

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    mode: String,
    cursor: Cursor,
    rows: Vec<String>,
    status: StatusMessage,
    #[serde(default)]
    status_position: StatusPosition,
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
        status_position: StatusPosition,
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
            status_position,
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

    pub fn position_label(&self) -> String {
        self.status_position.label()
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
#[serde(default)]
pub struct StatusPosition {
    pub column: u16,
    pub row: u16,
    pub total_rows: u16,
}

impl StatusPosition {
    pub fn label(&self) -> String {
        let column = self.column.saturating_add(1);
        let row = self.row.saturating_add(1);
        let total = self.total_rows.max(1);
        format!("{}:{}/{}", row, column, total)
    }
}

impl Default for StatusPosition {
    fn default() -> Self {
        Self {
            column: 0,
            row: 0,
            total_rows: 1,
        }
    }
}
