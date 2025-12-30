use std::io;

use crossterm::{
    cursor::MoveTo,
    style::Print,
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::{Frame, FrameRowSink};

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
        let mut painter = EditorRowsPainter::new(self.terminal);
        self.frame.rows().paint(self.usable_rows, &mut painter);
        painter.finish()
    }
}

struct EditorRowsPainter<'a> {
    terminal: &'a mut Terminal,
    result: io::Result<()>,
}

impl<'a> EditorRowsPainter<'a> {
    fn new(terminal: &'a mut Terminal) -> Self {
        Self {
            terminal,
            result: Ok(()),
        }
    }

    fn finish(self) -> io::Result<()> {
        self.result
    }
}

impl<'a> FrameRowSink for EditorRowsPainter<'a> {
    fn paint_row(&mut self, index: u16, row: &str) {
        if self.result.is_err() {
            return;
        }
        let screen_row = 1u16.saturating_add(index);
        self.result = self
            .terminal
            .queue_add_command(MoveTo(0, screen_row))
            .and_then(|_| self.terminal.queue_add_command(Clear(ClearType::CurrentLine)))
            .and_then(|_| self.terminal.queue_add_command(Print(row)));
    }
}
