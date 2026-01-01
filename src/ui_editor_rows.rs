use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::terminal::Terminal;
use vimrust_protocol::{Frame, FrameRowSink, RowSelection};

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
        self.frame.paint_rows(self.usable_rows, &mut painter);
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
    fn paint_row(&mut self, index: u16, row: &str, selection: RowSelection) {
        if self.result.is_err() {
            return;
        }
        let screen_row = 1u16.saturating_add(index);
        self.result = self.using_row(screen_row, row, selection);
    }
}

impl<'a> EditorRowsPainter<'a> {
    fn using_row(&mut self, screen_row: u16, row: &str, selection: RowSelection) -> io::Result<()> {
        self.terminal.queue_add_command(MoveTo(0, screen_row))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;

        match selection {
            RowSelection::None => {
                self.terminal.queue_add_command(Print(row))?;
            }
            RowSelection::Range { start, end } => {
                let mut prefix = String::new();
                let mut selected = String::new();
                let mut suffix = String::new();
                let mut idx = 0u16;
                for ch in row.chars() {
                    if idx < start {
                        prefix.push(ch);
                    } else if idx < end {
                        selected.push(ch);
                    } else {
                        suffix.push(ch);
                    }
                    idx = idx.saturating_add(1);
                }
                self.terminal.queue_add_command(Print(prefix))?;
                if !selected.is_empty() {
                    self.terminal
                        .queue_add_command(SetBackgroundColor(Color::DarkGrey))?;
                    self.terminal
                        .queue_add_command(SetForegroundColor(Color::White))?;
                    self.terminal.queue_add_command(Print(selected))?;
                    self.terminal.queue_add_command(ResetColor)?;
                }
                self.terminal.queue_add_command(Print(suffix))?;
            }
        }

        Ok(())
    }
}
