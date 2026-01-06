use std::io;

use crate::file::{File, FileChangeToken};
use crate::frame_signal::FrameSignal;
use vimrust_protocol::FilePath;
use vimrust_protocol::FrameSelection;
use vimrust_protocol::MoveDirection;
use vimrust_protocol::StatusMessage;

#[path = "editor_selection.rs"]
mod editor_selection;
#[path = "line_view.rs"]
mod line_view;

use self::editor_selection::{CursorUpdate, EditorSelection};
use self::line_view::LineView;
use crate::command_scope::CommandScope;

pub struct EditorVersion {
    label: &'static str,
}

impl EditorVersion {
    pub fn current() -> Self {
        Self { label: "0.1.0" }
    }

    pub fn append_to(&self, target: &mut String) {
        target.push_str(self.label);
    }
}

pub struct EditorView<'a> {
    file: &'a File,
    cursor_x: u16,
    cursor_y: u16,
    columns_offset: u16,
    rows_offset: u16,
}

impl<'a> EditorView<'a> {
    pub fn file_ref(&self) -> &File {
        self.file
    }

    pub fn cursor_column(&self) -> u16 {
        self.cursor_x
    }

    pub fn cursor_row(&self) -> u16 {
        self.cursor_y
    }

    pub fn column_offset(&self) -> u16 {
        self.columns_offset
    }

    pub fn row_offset(&self) -> u16 {
        self.rows_offset
    }
}

pub struct Editor {
    cursor_x: u16,
    cursor_y: u16,
    columns_offset: u16,
    rows_offset: u16,
    file: File,
    version: EditorVersion,
    selection: EditorSelection,
    line_view: LineView,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    column: u16,
    row: u16,
}

impl CursorPosition {
    pub fn new(column: u16, row: u16) -> Self {
        Self { column, row }
    }
}

pub struct FileChange {
    changed: bool,
}

impl FileChange {
    pub fn status_or(&self, status: &StatusMessage) -> StatusMessage {
        if self.changed {
            StatusMessage::Text {
                text: String::from("modified"),
            }
        } else {
            status.clone()
        }
    }
}

pub struct EditorSnapshot {
    cursor_x: u16,
    cursor_y: u16,
    columns_offset: u16,
    rows_offset: u16,
    change_mark: FileChangeToken,
}

impl EditorSnapshot {
    pub fn frame_signal(&self, editor: &Editor) -> FrameSignal {
        let same_cursor = self.cursor_x == editor.cursor_x && self.cursor_y == editor.cursor_y;
        let same_offsets =
            self.columns_offset == editor.columns_offset && self.rows_offset == editor.rows_offset;
        let same_change = self.change_mark == editor.file.change_mark();
        if same_cursor && same_offsets && same_change {
            FrameSignal::Skip
        } else {
            FrameSignal::Frame
        }
    }

    pub fn status_from(&self, editor: &Editor, status: StatusMessage) -> StatusMessage {
        if self.change_mark == editor.file.change_mark() {
            status
        } else {
            StatusMessage::Text {
                text: String::from("modified"),
            }
        }
    }
}

impl Editor {
    pub fn new(file: File) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            columns_offset: 0,
            rows_offset: 0,
            file,
            version: EditorVersion::current(),
            selection: EditorSelection::new(),
            line_view: LineView::new(4),
        }
    }

    /// Returns a read-only view with scroll offsets adjusted for the viewport.
    pub fn view_with_scroll(&self, number_of_columns: u16, number_of_rows: u16) -> EditorView<'_> {
        let (columns_offset, rows_offset) =
            self.scroll_offsets_compute(number_of_columns, number_of_rows);
        EditorView {
            file: &self.file,
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            columns_offset,
            rows_offset,
        }
    }

    pub fn file_read(&mut self) -> io::Result<()> {
        self.file.read()
    }

    pub fn file_save(&mut self, file_path: &mut FilePath) -> io::Result<String> {
        let result = self.file.save()?;
        *file_path = self.file.path();
        Ok(result)
    }

    pub fn file_path(&self) -> FilePath {
        self.file.path()
    }

    pub fn message_lock(&self) -> StatusMessage {
        self.file.message_lock()
    }

    pub fn file_lines_snapshot(&self) -> Vec<String> {
        self.file.lines_snapshot()
    }

    pub fn cursor_position(&self) -> CursorPosition {
        CursorPosition::new(self.cursor_x, self.cursor_y)
    }

    pub fn change_status(&self) -> FileChange {
        FileChange {
            changed: self.file.change_state(),
        }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            columns_offset: self.columns_offset,
            rows_offset: self.rows_offset,
            change_mark: self.file.change_mark(),
        }
    }

    pub fn visual_begin(&mut self) {
        let position = self.cursor_position();
        self.selection.begin(position);
    }

    pub fn visual_clear(&mut self) {
        self.selection.clear();
    }

    pub fn command_scope(&self) -> CommandScope {
        self.selection.scope()
    }

    pub fn selection_frame(
        &self,
        view: &EditorView<'_>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> FrameSelection {
        let cursor = self.cursor_position();
        self.selection.frame_selection(
            cursor,
            view,
            number_of_columns,
            number_of_rows,
            &self.file,
            &self.line_view,
        )
    }

    pub fn selection_case_kebab(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .kebab(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_camel(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .camel(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_snake(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .snake(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_screaming_snake(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .screaming_snake(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_pascal(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .pascal(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_train(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .train(position, &mut self.file, &self.line_view, &mut update);
    }

    pub fn selection_case_flat(&mut self) {
        let position = self.cursor_position();
        let mut update = CursorUpdate::new(&mut self.cursor_x, &mut self.cursor_y);
        self.selection
            .flat(position, &mut self.file, &self.line_view, &mut update);
    }

    fn scroll_offsets_compute(&self, number_of_columns: u16, number_of_rows: u16) -> (u16, u16) {
        if number_of_rows == 0 {
            return (self.columns_offset, self.rows_offset);
        }

        let mut rows_offset = self.rows_offset;
        let mut columns_offset = self.columns_offset;

        if self.cursor_y < rows_offset {
            rows_offset = self.cursor_y;
        }
        if self.cursor_y >= rows_offset.saturating_add(number_of_rows) {
            rows_offset = self
                .cursor_y
                .saturating_sub(number_of_rows)
                .saturating_add(1);
        }
        if self.cursor_x < columns_offset {
            columns_offset = self.cursor_x;
        }
        if self.cursor_x >= columns_offset.saturating_add(number_of_columns) {
            columns_offset = self
                .cursor_x
                .saturating_sub(number_of_columns)
                .saturating_add(1);
        }

        (columns_offset, rows_offset)
    }

    pub fn rows_render(
        &self,
        view: &EditorView<'_>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> Vec<String> {
        let mut rows = Vec::with_capacity(number_of_rows as usize);
        for row_number in 0..number_of_rows {
            let file_line_number = row_number.saturating_add(view.row_offset()) as usize;

            if file_line_number >= view.file_ref().line_count() {
                let mut line = String::from("~");
                let welcome =
                    self.welcome_line(view, number_of_columns, row_number, number_of_rows);
                line.push_str(&welcome);
                if line.len() > number_of_columns as usize {
                    line.truncate(number_of_columns as usize);
                }
                rows.push(line);
            } else if let Some(file_line) = view.file_ref().line_at(file_line_number) {
                let displayable_line = self.line_view.displayable_line(file_line);
                let visible_slice: String = displayable_line
                    .chars()
                    .skip(view.column_offset() as usize)
                    .take(number_of_columns as usize)
                    .collect();
                rows.push(visible_slice);
            }
        }

        rows
    }

    fn welcome_line(
        &self,
        view: &EditorView<'_>,
        number_of_columns: u16,
        row_number: u16,
        number_of_rows: u16,
    ) -> String {
        if row_number != number_of_rows / 3 {
            return String::new();
        }

        let file = view.file_ref();
        let mut has_text = false;
        let mut index = 0;
        let total = file.line_count();
        while index < total {
            if let Some(line) = file.line_at(index) {
                if line.len() > 0 {
                    has_text = true;
                    break;
                }
            }
            index = index.saturating_add(1);
        }

        if has_text {
            return String::new();
        }

        let mut welcome = String::from("VimRust -- version ");
        self.version.append_to(&mut welcome);
        if welcome.len() > number_of_columns as usize {
            welcome.truncate(number_of_columns as usize);
        }
        let padding = number_of_columns
            .saturating_sub(welcome.len() as u16)
            .saturating_div(2);
        let mut line = String::new();
        if padding > 1 {
            line.push_str(&" ".repeat(padding.saturating_sub(1) as usize));
        }
        line.push_str(&welcome);
        line
    }

    pub fn cursor_move(&mut self, direction: MoveDirection, usable_rows: u16) {
        let file_lines_len = self.file.line_count().min(u16::MAX as usize) as u16;

        match direction {
            MoveDirection::Left => {
                if let Some(line) = self.file.line_at(self.cursor_y as usize) {
                    self.cursor_x = self.line_view.column_previous_render(line, self.cursor_x);
                } else {
                    self.cursor_x = self.cursor_x.saturating_sub(1);
                }
            }
            MoveDirection::Right => {
                if let Some(line) = self.file.line_at(self.cursor_y as usize) {
                    self.cursor_x = self.line_view.column_next_render(line, self.cursor_x);
                } else {
                    self.cursor_x = self.cursor_x.saturating_add(1);
                }
            }
            MoveDirection::Home => {
                self.cursor_x = 0;
            }
            MoveDirection::End => {
                self.cursor_x = self.file_line_length(self.cursor_y);
            }
            MoveDirection::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            MoveDirection::Down => {
                self.cursor_y = self.cursor_y.saturating_add(1);
            }
            MoveDirection::PageUp => {
                if usable_rows == 0 {
                    self.cursor_y = 0;
                    self.rows_offset = 0;
                } else {
                    let new_cursor_y = self.cursor_y.saturating_sub(usable_rows);
                    let lower_third = usable_rows.saturating_mul(2).saturating_div(3);
                    let new_offset = new_cursor_y.saturating_sub(lower_third);
                    self.cursor_y = new_cursor_y;
                    self.rows_offset = new_offset;
                }
            }
            MoveDirection::PageDown => {
                if usable_rows == 0 {
                    self.cursor_y = file_lines_len;
                } else {
                    let new_cursor_y = self
                        .cursor_y
                        .saturating_add(usable_rows)
                        .min(file_lines_len);
                    let upper_third = usable_rows.saturating_div(3);
                    let new_offset = new_cursor_y.saturating_sub(upper_third);
                    self.cursor_y = new_cursor_y;
                    self.rows_offset = new_offset;
                }
            }
        }

        if self.cursor_y > file_lines_len {
            self.cursor_y = file_lines_len;
        }

        let line_length = self.file_line_length(self.cursor_y);
        if self.cursor_x > line_length {
            self.cursor_x = line_length;
        }
        if let Some(line) = self.file.line_at(self.cursor_y as usize) {
            self.cursor_x = self
                .line_view
                .snap_cursor_to_render_character(line, self.cursor_x);
        }
    }

    pub fn cursor_place(&mut self, position: CursorPosition) {
        self.cursor_x = position.column;
        self.cursor_y = position.row;

        let max_row = self
            .file
            .line_count()
            .saturating_sub(1)
            .min(u16::MAX as usize) as u16;
        if self.cursor_y > max_row {
            self.cursor_y = max_row;
        }

        let line_length = self.file_line_length(self.cursor_y);
        if self.cursor_x > line_length {
            self.cursor_x = line_length;
        }
        if let Some(line) = self.file.line_at(self.cursor_y as usize) {
            self.cursor_x = self
                .line_view
                .snap_cursor_to_render_character(line, self.cursor_x);
        }
    }

    pub fn char_insert(&mut self, ch: char) {
        let target_line = self.cursor_y as usize;
        self.file.line_ensure(target_line);

        let insert_at = match self.file.line_at(target_line) {
            Some(line) => self
                .line_view
                .column_to_char_index_render(line, self.cursor_x),
            None => 0,
        };
        let advance = self.line_view.char_render_width(ch, self.cursor_x);

        if let Some(line) = self.file.line_at_mut(target_line) {
            let previous_len = line.len();
            let previous_cursor_x = self.cursor_x;
            line.insert(insert_at, ch);
            self.cursor_x = self.cursor_x.saturating_add(advance);
            if line.len() != previous_len || self.cursor_x != previous_cursor_x {
                self.file.touch();
            }
            return;
        }
    }

    pub fn line_break(&mut self) {
        let target_line = self.cursor_y as usize;
        self.file.line_ensure(target_line);

        let split_at = match self.file.line_at(target_line) {
            Some(line) => self
                .line_view
                .column_to_char_index_render(line, self.cursor_x),
            None => 0,
        };

        let remainder = match self.file.line_at_mut(target_line) {
            Some(line) => line.split_off(split_at),
            None => String::new(),
        };

        self.file
            .line_insert(target_line.saturating_add(1), remainder);
        self.cursor_y = self.cursor_y.saturating_add(1);
        self.cursor_x = 0;
        self.file.touch();
    }

    pub fn backspace_delete(&mut self) {
        if self.cursor_x == 0 && self.cursor_y == 0 {
            return;
        }

        if self.cursor_x == 0 {
            let current_index = self.cursor_y as usize;
            if current_index == 0 || current_index >= self.file.line_count() {
                return;
            }
            if let Some(current_line) = self.file.line_at(current_index).cloned() {
                let new_cursor_x = match self.file.line_at(current_index.saturating_sub(1)) {
                    Some(prev) => self.line_view.visual_line_length(prev),
                    None => 0,
                };
                if let Some(previous_line) = self.file.line_at_mut(current_index.saturating_sub(1))
                {
                    previous_line.push_str(&current_line);
                    self.file.line_remove(current_index);
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                    self.cursor_x = new_cursor_x;
                    self.file.touch();
                    return;
                }
            }
            return;
        }

        let (new_cursor_x, delete_idx) = match self.file.line_at(self.cursor_y as usize) {
            Some(line) => (
                self.line_view.column_previous_render(line, self.cursor_x),
                self.line_view.column_to_char_index_render(
                    line,
                    self.line_view.column_previous_render(line, self.cursor_x),
                ),
            ),
            None => (0, 0),
        };

        if let Some(line) = self.file.line_at_mut(self.cursor_y as usize)
            && delete_idx < line.len()
        {
            line.remove(delete_idx);
            self.cursor_x = new_cursor_x;
            self.file.touch();
            return;
        }
    }

    pub fn under_cursor_delete(&mut self) {
        let delete_idx = match self.file.line_at(self.cursor_y as usize) {
            Some(line) => self
                .line_view
                .column_to_char_index_render(line, self.cursor_x),
            None => return,
        };

        if let Some(line) = self.file.line_at_mut(self.cursor_y as usize)
            && delete_idx < line.len()
        {
            line.remove(delete_idx);
            self.file.touch();
            return;
        }

        let current_index = self.cursor_y as usize;
        if current_index + 1 < self.file.line_count()
            && let Some(next_line) = self.file.line_at(current_index + 1).cloned()
            && let Some(current_line) = self.file.line_at_mut(current_index)
        {
            current_line.push_str(&next_line);
            self.file.line_remove(current_index + 1);
            self.file.touch();
            return;
        }
    }

    pub fn snap_cursor_to_tab_start(&mut self) {
        if let Some(line) = self.file.line_at(self.cursor_y as usize)
            && let Some(start) = self.line_view.tab_segment_start(line, self.cursor_x)
        {
            self.cursor_x = start;
        }
    }

    fn file_line_length(&self, cursor_y: u16) -> u16 {
        let line_index = cursor_y as usize;
        if line_index >= self.file.line_count() {
            return 0;
        }

        if let Some(line) = self.file.line_at(line_index) {
            return self.line_view.line_length(line);
        }

        0
    }
}

#[cfg(test)]
impl Editor {
    pub fn file_lines_replace(&mut self, lines: Vec<String>) {
        self.file.lines_replace(lines);
    }

    pub fn cursor_position_store(&mut self, position: CursorPosition) {
        self.cursor_x = position.column;
        self.cursor_y = position.row;
    }
}
