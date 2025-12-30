use std::io;

use crate::{
    mode::EditorMode,
    terminal::Terminal,
    ui_prompt_list::PromptListView,
    ui_editor_rows::EditorRowsPanel,
};
use vimrust_protocol::{CommandUiAccess, CursorSink, Frame};
use crossterm::cursor::MoveTo;

pub(crate) struct UiBody<'a> {
    mode: EditorMode,
    frame: &'a Frame,
    number_of_columns: u16,
    usable_rows: u16,
}

impl<'a> UiBody<'a> {
    pub(crate) fn new(
        mode: EditorMode,
        frame: &'a Frame,
        number_of_columns: u16,
        usable_rows: u16,
    ) -> Self {
        Self {
            mode,
            frame,
            number_of_columns,
            usable_rows,
        }
    }

    pub(crate) fn paint(&self, terminal: &mut Terminal) -> io::Result<()> {
        if self.usable_rows == 0 {
            return Ok(());
        }

        match self.mode {
            EditorMode::PromptCommand | EditorMode::PromptKeymap => {
                match self.frame.command_ui() {
                    CommandUiAccess::Available(cmd_ui) => {
                        let mut panel = PromptListView::new(
                            terminal,
                            &cmd_ui,
                            self.number_of_columns,
                            1,
                            self.usable_rows,
                        );
                        panel.paint()?;
                    }
                    CommandUiAccess::Missing => {}
                }
            }
            _ => {
                let mut panel = EditorRowsPanel::new(terminal, self.frame, self.usable_rows);
                panel.paint()?;
            }
        }

        Ok(())
    }
}

pub(crate) struct CursorPlacement<'a> {
    mode: EditorMode,
    frame: &'a Frame,
    number_of_columns: u16,
    number_of_rows: u16,
}

impl<'a> CursorPlacement<'a> {
    pub(crate) fn new(
        mode: EditorMode,
        frame: &'a Frame,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> Self {
        Self {
            mode,
            frame,
            number_of_columns,
            number_of_rows,
        }
    }

    pub(crate) fn command(&self) -> MoveTo {
        match self.mode {
            EditorMode::PromptCommand | EditorMode::PromptKeymap => {
                match self.frame.command_ui() {
                    CommandUiAccess::Available(cmd_ui) => {
                        let mut command_cursor = (
                            cmd_ui
                                .cursor_column()
                                .saturating_add(1)
                                .min(self.number_of_columns.saturating_sub(1)),
                            0,
                        );
                        if cmd_ui.list_focus() {
                            if let Some(selected) = cmd_ui.selected_item() {
                                let relative_row =
                                    selected.saturating_sub(cmd_ui.scroll_position()) as u16;
                                let list_row = 1u16
                                    .saturating_add(1)
                                    .saturating_add(2)
                                    .saturating_add(relative_row)
                                    .min(self.number_of_rows.saturating_sub(1));
                                command_cursor = (0, list_row);
                            }
                        }
                        MoveTo(command_cursor.0, command_cursor.1)
                    }
                    CommandUiAccess::Missing => MoveTo(0, 0),
                }
            }
            _ => {
                let mut target =
                    CursorTarget::new(self.number_of_columns, self.number_of_rows);
                self.frame.cursor().place_on(&mut target);
                target.command()
            }
        }
    }
}

struct CursorTarget {
    number_of_columns: u16,
    number_of_rows: u16,
    column: u16,
    row: u16,
    has_cursor: bool,
}

impl CursorTarget {
    fn new(number_of_columns: u16, number_of_rows: u16) -> Self {
        Self {
            number_of_columns,
            number_of_rows,
            column: 0,
            row: 0,
            has_cursor: false,
        }
    }

    fn command(&self) -> MoveTo {
        if !self.has_cursor {
            return MoveTo(0, 0);
        }
        let cursor_col = self.column.min(self.number_of_columns.saturating_sub(1));
        let base_row = self.row;
        // Keep the edit cursor off the command-line row (row 0).
        let min_editor_row = 1;
        let max_editor_row = self.number_of_rows.saturating_sub(1).max(min_editor_row);
        let cursor_row = base_row.max(1).min(max_editor_row);
        MoveTo(cursor_col, cursor_row)
    }
}

impl CursorSink for CursorTarget {
    fn place(&mut self, column: u16, row: u16) {
        self.column = column;
        self.row = row;
        self.has_cursor = true;
    }
}
