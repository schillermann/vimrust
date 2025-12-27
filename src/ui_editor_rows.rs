use std::io;

use crossterm::{
    cursor::MoveTo,
    style::Print,
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::Frame;

pub(crate) struct EditorRowsPanel<'a> {
    terminal: &'a mut Terminal,
    frame: &'a Frame,
    usable_rows: u16,
}

impl<'a> EditorRowsPanel<'a> {
    pub(crate) fn new(terminal: &'a mut Terminal, frame: &'a Frame, usable_rows: u16) -> Self {
        Self {
            terminal,
            frame,
            usable_rows,
        }
    }

    pub(crate) fn paint(&mut self) -> io::Result<()> {
        let rows = self.frame.editor_rows();
        let mut idx = 0usize;
        while idx < rows.len() {
            if idx as u16 >= self.usable_rows {
                break;
            }
            let row = &rows[idx];
            let screen_row = 1u16.saturating_add(idx as u16);
            self.terminal.queue_add_command(MoveTo(0, screen_row))?;
            self.terminal
                .queue_add_command(Clear(ClearType::CurrentLine))?;
            self.terminal.queue_add_command(Print(row))?;
            idx = idx.saturating_add(1);
        }
        Ok(())
    }
}
