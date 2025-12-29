use std::io;

use crate::{
    mode::EditorMode,
    terminal::Terminal,
    ui_prompt_list::PromptListView,
    ui_editor_rows::EditorRowsPanel,
};
use vimrust_protocol::Frame;

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
                if let Some(cmd_ui) = self.frame.command_ui_frame() {
                    let mut panel = PromptListView::new(
                        terminal,
                        cmd_ui,
                        self.number_of_columns,
                        1,
                        self.usable_rows,
                    );
                    panel.paint()?;
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

    pub(crate) fn position(&self) -> (u16, u16) {
        match self.mode {
            EditorMode::PromptCommand | EditorMode::PromptKeymap => {
                if let Some(cmd_ui) = self.frame.command_ui_frame() {
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
                    command_cursor
                } else {
                    (0, 0)
                }
            }
            _ => {
                let cursor = self.frame.cursor_position();
                let cursor_col = cursor
                    .column_index()
                    .min(self.number_of_columns.saturating_sub(1));
                let base_row = cursor.row_index();
                // Keep the edit cursor off the command-line row (row 0).
                let min_editor_row = 1;
                let max_editor_row = self.number_of_rows.saturating_sub(1).max(min_editor_row);
                let cursor_row = base_row.max(1).min(max_editor_row);
                (cursor_col, cursor_row)
            }
        }
    }
}
