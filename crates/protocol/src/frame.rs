use serde::{Deserialize, Serialize};

use crate::{CommandUiFrame, FilePath, ProtocolVersion, StatusMessage};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrameMode {
    Normal,
    Edit,
    Visual,
    PromptCommand,
    PromptKeymap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    mode: FrameMode,
    cursor: Cursor,
    rows: FrameRows,
    status: StatusMessage,
    #[serde(default)]
    status_position: StatusPosition,
    file_path: FilePath,
    size: Viewport,
    command_ui: Option<CommandUiFrame>,
    #[serde(default)]
    selection: FrameSelection,
    #[serde(default)]
    protocol_version: ProtocolVersion,
}

impl Frame {
    pub fn new(
        mode: FrameMode,
        cursor: Cursor,
        rows: Vec<String>,
        status: StatusMessage,
        status_position: StatusPosition,
        file_path: FilePath,
        size: (u16, u16),
        command_ui: Option<CommandUiFrame>,
        selection: FrameSelection,
        protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            mode,
            cursor,
            rows: FrameRows::new(rows),
            status,
            status_position,
            file_path,
            size: Viewport::new(size.0, size.1),
            command_ui,
            selection,
            protocol_version,
        }
    }

    pub fn mode(&self) -> FrameMode {
        self.mode
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor.clone()
    }

    pub fn rows(&self) -> FrameRows {
        self.rows.clone()
    }

    pub fn paint_rows(&self, usable_rows: u16, sink: &mut dyn FrameRowSink) {
        self.rows.paint(usable_rows, &self.selection, sink);
    }

    pub fn status(&self) -> StatusMessage {
        self.status.clone()
    }

    pub fn position(&self) -> StatusPosition {
        self.status_position.clone()
    }

    pub fn path(&self) -> FilePath {
        self.file_path.clone()
    }

    pub fn viewport(&self) -> Viewport {
        self.size.clone()
    }

    pub fn command_ui(&self) -> CommandUiAccess {
        match self.command_ui.as_ref() {
            Some(frame) => CommandUiAccess::Available(frame.clone()),
            None => CommandUiAccess::Missing,
        }
    }

    pub fn protocol(&self) -> ProtocolVersion {
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

    pub fn place_on(&self, sink: &mut dyn CursorSink) {
        sink.place(self.col, self.row);
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct StatusPosition {
    column: u16,
    row: u16,
    total_rows: u16,
}

impl StatusPosition {
    pub fn new(column: u16, row: u16, total_rows: u16) -> Self {
        Self {
            column,
            row,
            total_rows,
        }
    }

    pub fn append_to(&self, target: &mut String) {
        let column = self.column.saturating_add(1);
        let row = self.row.saturating_add(1);
        let total = self.total_rows.max(1);
        target.push_str(&format!("{}:{}/{}", row, column, total));
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct FrameRows(Vec<String>);

impl FrameRows {
    pub fn new(rows: Vec<String>) -> Self {
        Self(rows)
    }

    pub fn paint(&self, usable_rows: u16, selection: &FrameSelection, sink: &mut dyn FrameRowSink) {
        let mut idx = 0usize;
        while idx < self.0.len() {
            if idx as u16 >= usable_rows {
                break;
            }
            let row = &self.0[idx];
            let row_len = row.chars().count().min(u16::MAX as usize) as u16;
            let row_selection = selection.row_span(idx as u16, row_len);
            sink.paint_row(idx as u16, row, row_selection);
            idx = idx.saturating_add(1);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Viewport(u16, u16);

impl Viewport {
    pub fn new(columns: u16, rows: u16) -> Self {
        Self(columns, rows)
    }

    pub fn apply_to(&self, sink: &mut dyn ViewportSink) {
        sink.size(self.0, self.1);
    }
}

pub enum CommandUiAccess {
    Available(CommandUiFrame),
    Missing,
}

pub trait FrameRowSink {
    fn paint_row(&mut self, index: u16, row: &str, selection: RowSelection);
}

pub trait ViewportSink {
    fn size(&mut self, columns: u16, rows: u16);
}

pub trait CursorSink {
    fn place(&mut self, column: u16, row: u16);
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrameSelection {
    None,
    Range { start: Cursor, end: Cursor },
}

impl FrameSelection {
    pub fn none() -> Self {
        Self::None
    }

    pub fn range(start: Cursor, end: Cursor) -> Self {
        Self::Range { start, end }
    }

    pub fn row_span(&self, row: u16, row_len: u16) -> RowSelection {
        match self {
            FrameSelection::None => RowSelection::None,
            FrameSelection::Range { start, end } => {
                let start_row = start.row;
                let end_row = end.row;
                if row < start_row || row > end_row {
                    return RowSelection::None;
                }

                let mut start_col = 0u16;
                let mut end_col = row_len;
                if start_row == end_row {
                    start_col = start.col;
                    end_col = end.col;
                } else if row == start_row {
                    start_col = start.col;
                    end_col = row_len;
                } else if row == end_row {
                    start_col = 0;
                    end_col = end.col;
                }

                if start_col > row_len {
                    start_col = row_len;
                }
                if end_col > row_len {
                    end_col = row_len;
                }
                if end_col <= start_col {
                    RowSelection::None
                } else {
                    RowSelection::Range {
                        start: start_col,
                        end: end_col,
                    }
                }
            }
        }
    }
}

impl Default for FrameSelection {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RowSelection {
    None,
    Range { start: u16, end: u16 },
}
