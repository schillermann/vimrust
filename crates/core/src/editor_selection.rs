use crate::command_scope::CommandScope;
use crate::editor::{CursorPosition, EditorView, LineView};
use crate::file::File;
use vimrust_protocol::FrameSelection;

#[path = "editor_selection_range.rs"]
mod editor_selection_range;
#[path = "editor_selection_state.rs"]
mod editor_selection_state;
#[path = "editor_selection_transform.rs"]
mod editor_selection_transform;

use self::editor_selection_state::VisualSelection;
use self::editor_selection_transform::{
    CamelCaseTransform, KebabCaseTransform, PascalCaseTransform, ScreamingSnakeCaseTransform,
    SnakeCaseTransform, TrainCaseTransform,
};

pub(super) struct EditorSelection {
    visual: VisualSelection,
}

impl EditorSelection {
    pub(super) fn new() -> Self {
        Self {
            visual: VisualSelection::new(),
        }
    }

    pub(super) fn begin(&mut self, position: CursorPosition) {
        self.visual.begin(position);
    }

    pub(super) fn clear(&mut self) {
        self.visual.clear();
    }

    pub(super) fn scope(&self) -> CommandScope {
        self.visual.scope()
    }

    pub(super) fn frame_selection(
        &self,
        cursor: CursorPosition,
        view: &EditorView<'_>,
        number_of_columns: u16,
        number_of_rows: u16,
        file: &File,
        line_view: &LineView,
    ) -> FrameSelection {
        if matches!(self.visual.scope(), CommandScope::Normal) {
            return FrameSelection::none();
        }
        let range = self.visual.range_for(cursor, file, line_view);
        range.frame_selection(view, number_of_columns, number_of_rows)
    }

    pub(super) fn kebab(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = KebabCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }

    pub(super) fn camel(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = CamelCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }

    pub(super) fn snake(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = SnakeCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }

    pub(super) fn screaming_snake(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = ScreamingSnakeCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }

    pub(super) fn pascal(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = PascalCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }

    pub(super) fn train(
        &mut self,
        cursor: CursorPosition,
        file: &mut File,
        line_view: &LineView,
        cursor_update: &mut CursorUpdate<'_>,
    ) {
        let range = self.visual.range_for(cursor, file, line_view);
        let transform = TrainCaseTransform;
        range.apply_to(file, line_view, cursor_update, &transform);
        self.visual.clear();
    }
}

pub(super) struct CursorUpdate<'a> {
    column: &'a mut u16,
    row: &'a mut u16,
}

impl<'a> CursorUpdate<'a> {
    pub(super) fn new(column: &'a mut u16, row: &'a mut u16) -> Self {
        Self { column, row }
    }

    pub(super) fn place(&mut self, position: CursorPosition, file: &File, line_view: &LineView) {
        *self.column = position.column;
        *self.row = position.row;
        if let Some(line) = file.line_at(*self.row as usize) {
            *self.column = line_view.snap_cursor_to_render_character(line, *self.column);
        }
    }
}
