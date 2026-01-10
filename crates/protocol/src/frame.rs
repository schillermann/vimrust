use serde::{Deserialize, Serialize};

use crate::{DocumentFile, PromptUiFrame, ProtocolVersion, StatusMessage};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FrameEditorMode {
    Normal,
    Edit,
    Visual,
    PromptCommand,
    PromptKeymap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Frame {
    mode: FrameEditorMode,
    cursor: Cursor,
    rows: FrameRows,
    status: StatusMessage,
    #[serde(default)]
    status_position: StatusPosition,
    file_path: DocumentFile,
    size: Viewport,
    #[serde(default)]
    command_ui: CommandUiSlot,
    #[serde(default)]
    selection: FrameSelection,
    #[serde(default)]
    protocol_version: ProtocolVersion,
}

impl Frame {
    pub fn new(
        mode: FrameEditorMode,
        cursor: Cursor,
        rows: Vec<String>,
        status: StatusMessage,
        status_position: StatusPosition,
        file_path: DocumentFile,
        size: (u16, u16),
        command_ui: CommandUiSlot,
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

    pub fn empty() -> Self {
        Self {
            mode: FrameEditorMode::Normal,
            cursor: Cursor::new(0, 0),
            rows: FrameRows::new(Vec::new()),
            status: StatusMessage::Empty,
            status_position: StatusPosition::default(),
            file_path: DocumentFile { path: String::new() },
            size: Viewport::new(0, 0),
            command_ui: CommandUiSlot::missing(),
            selection: FrameSelection::none(),
            protocol_version: ProtocolVersion::current(),
        }
    }

    pub fn mode(&self) -> FrameEditorMode {
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

    pub fn path(&self) -> DocumentFile {
        self.file_path.clone()
    }

    pub fn viewport(&self) -> Viewport {
        self.size.clone()
    }

    pub fn command_ui(&self) -> CommandUiAccess {
        self.command_ui.access()
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
    Available(PromptUiFrame),
    Missing,
}

#[derive(Clone)]
pub struct CommandUiSlot {
    frame: PromptUiFrame,
    visible: bool,
}

impl CommandUiSlot {
    pub fn available(frame: PromptUiFrame) -> Self {
        Self {
            frame,
            visible: true,
        }
    }

    pub fn missing() -> Self {
        Self {
            frame: PromptUiFrame::empty(),
            visible: false,
        }
    }

    fn access(&self) -> CommandUiAccess {
        if self.visible {
            CommandUiAccess::Available(self.frame.clone())
        } else {
            CommandUiAccess::Missing
        }
    }
}

impl Default for CommandUiSlot {
    fn default() -> Self {
        Self::missing()
    }
}

impl Serialize for CommandUiSlot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.visible {
            self.frame.serialize(serializer)
        } else {
            serializer.serialize_none()
        }
    }
}

impl<'de> Deserialize<'de> for CommandUiSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let slot = Option::<PromptUiFrame>::deserialize(deserializer)?;
        Ok(match slot {
            Some(frame) => CommandUiSlot::available(frame),
            None => CommandUiSlot::missing(),
        })
    }
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
