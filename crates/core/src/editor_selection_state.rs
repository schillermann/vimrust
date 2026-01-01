use crate::command_scope::CommandScope;
use crate::editor::{CursorPosition, LineView};
use crate::file::File;

use super::editor_selection_range::{selection_end, SelectionRange};

impl CursorPosition {
    fn order_to(&self, other: CursorPosition) -> PositionOrder {
        if self.row < other.row {
            PositionOrder::Earlier
        } else if self.row > other.row {
            PositionOrder::Later
        } else if self.column < other.column {
            PositionOrder::Earlier
        } else if self.column > other.column {
            PositionOrder::Later
        } else {
            PositionOrder::Same
        }
    }
}

#[derive(Copy, Clone)]
enum PositionOrder {
    Earlier,
    Later,
    Same,
}

pub(super) enum VisualSelection {
    Idle,
    Active(SelectionAnchor),
}

impl VisualSelection {
    pub(super) fn new() -> Self {
        Self::Idle
    }

    pub(super) fn begin(&mut self, position: CursorPosition) {
        *self = Self::Active(SelectionAnchor::new(position));
    }

    pub(super) fn clear(&mut self) {
        *self = Self::Idle;
    }

    pub(super) fn scope(&self) -> CommandScope {
        match self {
            Self::Idle => CommandScope::Normal,
            Self::Active(_) => CommandScope::Visual,
        }
    }

    pub(super) fn range_for(
        &self,
        cursor: CursorPosition,
        file: &File,
        line_view: &LineView,
    ) -> SelectionRange {
        match self {
            Self::Idle => SelectionRange::empty(cursor),
            Self::Active(anchor) => anchor.range_to(cursor, file, line_view),
        }
    }
}

pub(super) struct SelectionAnchor {
    position: CursorPosition,
}

impl SelectionAnchor {
    fn new(position: CursorPosition) -> Self {
        Self { position }
    }

    fn range_to(&self, cursor: CursorPosition, file: &File, line_view: &LineView) -> SelectionRange {
        match self.position.order_to(cursor) {
            PositionOrder::Earlier => {
                SelectionRange::new(self.position, selection_end(file, line_view, cursor))
            }
            PositionOrder::Later => {
                SelectionRange::new(cursor, selection_end(file, line_view, self.position))
            }
            PositionOrder::Same => {
                SelectionRange::new(cursor, selection_end(file, line_view, cursor))
            }
        }
    }
}
