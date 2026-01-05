use std::io;

use crossterm::style::{Attribute, Color, Print, SetAttribute, SetForegroundColor};

use crate::terminal::Terminal;
use vimrust_protocol::PromptInputSelection;

pub(crate) struct PromptLineHighlight {
    selection: PromptInputSelection,
}

impl PromptLineHighlight {
    pub(crate) fn new(selection: PromptInputSelection) -> Self {
        Self { selection }
    }

    pub(crate) fn visible_indices(&self, content: &str, inner_width: usize) -> Vec<usize> {
        let indices = self.selection.indices();
        if indices.is_empty() {
            return indices;
        }
        let visible_len = content.chars().take(inner_width).count();
        let mut visible = Vec::new();
        let mut idx = 0usize;
        while idx < indices.len() {
            let position = indices[idx];
            if position < visible_len {
                visible.push(position.saturating_add(1));
            }
            idx = idx.saturating_add(1);
        }
        visible
    }

    pub(crate) fn queue(
        terminal: &mut Terminal,
        line: &str,
        default_fg: Color,
        highlight_indices: Vec<usize>,
    ) -> io::Result<()> {
        let highlight_fg = Color::White;
        terminal.queue_add_command(SetForegroundColor(default_fg))?;
        let mut match_pos = 0usize;
        let mut next_match = if highlight_indices.is_empty() {
            usize::MAX
        } else {
            highlight_indices[0]
        };
        let mut idx = 0usize;
        for ch in line.chars() {
            if idx == next_match {
                terminal.queue_add_command(SetAttribute(Attribute::Italic))?;
                terminal.queue_add_command(SetForegroundColor(highlight_fg))?;
                terminal.queue_add_command(Print(ch))?;
                terminal.queue_add_command(SetAttribute(Attribute::Reset))?;
                terminal.queue_add_command(SetForegroundColor(default_fg))?;
                match_pos = match_pos.saturating_add(1);
                if match_pos < highlight_indices.len() {
                    next_match = highlight_indices[match_pos];
                } else {
                    next_match = usize::MAX;
                }
            } else {
                terminal.queue_add_command(Print(ch))?;
            }
            idx = idx.saturating_add(1);
        }
        Ok(())
    }
}
