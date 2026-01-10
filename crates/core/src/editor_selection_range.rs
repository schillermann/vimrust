use crate::editor::{CursorPosition, EditorView, LineView};
use crate::file::File;
use vimrust_protocol::{Cursor, FrameSelection};

use super::CursorUpdate;
use super::editor_selection_transform::SelectionTransform;

pub(super) struct SelectionRange {
    start: CursorPosition,
    end: CursorPosition,
}

impl SelectionRange {
    pub(super) fn new(start: CursorPosition, end: CursorPosition) -> Self {
        Self { start, end }
    }

    pub(super) fn empty(position: CursorPosition) -> Self {
        Self {
            start: position,
            end: position,
        }
    }

    pub(super) fn frame_selection(
        &self,
        view: &EditorView<'_>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> FrameSelection {
        if number_of_columns == 0 || number_of_rows == 0 {
            return FrameSelection::none();
        }
        let view_top = view.row_offset();
        let view_bottom = view_top.saturating_add(number_of_rows.saturating_sub(1));
        if self.end.row < view_top || self.start.row > view_bottom {
            return FrameSelection::none();
        }

        let start_before_view = self.start.row < view_top;
        let end_after_view = self.end.row > view_bottom;

        let max_row = number_of_rows.saturating_sub(1);
        let mut start_row = self.start.row.saturating_sub(view_top);
        let mut end_row = self.end.row.saturating_sub(view_top);
        if start_row > max_row {
            start_row = max_row;
        }
        if end_row > max_row {
            end_row = max_row;
        }

        let max_col = number_of_columns;
        let mut start_col = self.start.column.saturating_sub(view.column_offset());
        let mut end_col = self.end.column.saturating_sub(view.column_offset());
        if start_before_view {
            start_col = 0;
        }
        if end_after_view {
            end_col = max_col;
        }
        if start_col > max_col {
            start_col = max_col;
        }
        if end_col > max_col {
            end_col = max_col;
        }

        FrameSelection::range(
            Cursor::new(start_col, start_row),
            Cursor::new(end_col, end_row),
        )
    }

    pub(super) fn apply_to(
        &self,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
        transform: &dyn SelectionTransform,
    ) {
        let last_row = file.line_count().saturating_sub(1);
        let mut start_row = self.start.row as usize;
        let mut end_row = self.end.row as usize;
        if start_row > last_row {
            start_row = last_row;
        }
        if end_row > last_row {
            end_row = last_row;
        }

        let start_line = file.line_at(start_row).to_string();
        let end_line = file.line_at(end_row).to_string();

        let start_idx = line_view.column_to_char_index_render(&start_line, self.start.column);
        let end_idx = line_view.column_to_char_index_render(&end_line, self.end.column);

        if start_row == end_row {
            let prefix = start_line[..start_idx].to_string();
            let selected = start_line[start_idx..end_idx].to_string();
            let suffix = start_line[end_idx..].to_string();
            let replacement = transform.transform(&selected);
            let mut updated = String::new();
            updated.push_str(&prefix);
            updated.push_str(&replacement);
            updated.push_str(&suffix);
            let mut changed = false;
            let line = file.line_at_mut(start_row);
            if *line != updated {
                *line = updated;
                changed = true;
            }
            if changed {
                file.touch();
            }
            cursor_update.place(
                CursorPosition::new(self.start.column, start_row as u16),
                file,
                line_view,
            );
            return;
        }

        let mut selected = String::new();
        selected.push_str(&start_line[start_idx..]);
        selected.push('\n');
        let mut mid_row = start_row.saturating_add(1);
        while mid_row < end_row {
            selected.push_str(file.line_at(mid_row));
            selected.push('\n');
            mid_row = mid_row.saturating_add(1);
        }
        selected.push_str(&end_line[..end_idx]);

        let replacement = transform.transform(&selected);
        let prefix = start_line[..start_idx].to_string();
        let suffix = end_line[end_idx..].to_string();
        let mut updated = String::new();
        updated.push_str(&prefix);
        updated.push_str(&replacement);
        updated.push_str(&suffix);

        let mut changed = false;
        let line = file.line_at_mut(start_row);
        if *line != updated {
            *line = updated;
            changed = true;
        }

        let mut remove_count = end_row.saturating_sub(start_row);
        while remove_count > 0 {
            let remove_at = start_row.saturating_add(1);
            if remove_at < file.line_count() {
                file.line_remove(remove_at);
                changed = true;
            }
            remove_count = remove_count.saturating_sub(1);
        }

        if changed {
            file.touch();
        }

        cursor_update.place(
            CursorPosition::new(self.start.column, start_row as u16),
            file,
            line_view,
        );
    }
}

pub(super) fn selection_end(
    file: &File,
    line_view: &LineView,
    position: CursorPosition,
) -> CursorPosition {
    let row = position.row;
    let row_index = row as usize;
    let column = if row_index < file.line_count() {
        selection_end_for_line(line_view, file.line_at(row_index), position.column)
    } else {
        position.column
    };
    CursorPosition::new(column, row)
}

fn selection_end_for_line(line_view: &LineView, line: &str, cursor_x: u16) -> u16 {
    let mut column: u16 = 0;
    for ch in line.chars() {
        let width = line_view.char_render_width(ch, column);
        let start = column;
        let end = column.saturating_add(width);
        if cursor_x >= start && cursor_x < end {
            return end;
        }
        column = end;
    }
    cursor_x
}
