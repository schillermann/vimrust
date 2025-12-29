use std::io::{self, Stdout, Write};

use crossterm::{
    Command,
    cursor::{MoveTo, SetCursorStyle},
    execute, queue,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

use crate::{buffer::Buffer, mode::EditorMode};

pub struct Terminal {
    size: (u16, u16),
    out: Stdout,
    buffer: Buffer,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = std::io::stdout();
        execute!(out, EnterAlternateScreen)?;
        let size = size()?;
        Ok(Self {
            size,
            out,
            buffer: Buffer::new(),
        })
    }

    pub fn cleanup(&mut self) {
        let _ = execute!(
            self.out,
            Clear(ClearType::All),
            MoveTo(0, 0),
            SetCursorStyle::DefaultUserShape,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }

    pub fn size_update(&mut self) -> io::Result<()> {
        self.size = size()?;
        Ok(())
    }

    pub fn size(&self) -> (u16, u16) {
        self.size
    }

    pub fn set_cursor_style(&mut self, mode: &EditorMode) -> io::Result<()> {
        let style = match mode {
            EditorMode::Normal => SetCursorStyle::DefaultUserShape,
            EditorMode::Edit => SetCursorStyle::SteadyBar,
            EditorMode::PromptCommand => SetCursorStyle::SteadyBar,
            EditorMode::PromptKeymap => SetCursorStyle::SteadyBar,
        };
        execute!(self.out, style)
    }

    pub(crate) fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    pub fn queue_add_command<C: Command>(&mut self, command: C) -> io::Result<()> {
        let writer = self.buffer.writer();
        queue!(writer, command)?;
        Ok(())
    }

    pub fn queue_execute(&mut self) -> io::Result<()> {
        if self.buffer.changed() {
            self.out.write_all(self.buffer.slice())?;
            self.out.flush()?;
            self.buffer.clear();
        }
        Ok(())
    }
}
