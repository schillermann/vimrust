use std::io;

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

use crate::{
    mode::EditorMode, terminal::Terminal, ui_prompt_line_focus::PromptLineFocus,
    ui_prompt_line_highlight::PromptLineHighlight,
    ui_prompt_line_placeholder::PromptLinePlaceholder,
};
use vimrust_protocol::PromptInputSelection;

pub(crate) struct PromptLine<'a, 'b> {
    terminal: &'a mut Terminal,
    number_of_columns: u16,
    content: &'b str,
    selection: PromptInputSelection,
    focus: PromptLineFocus,
    mode: EditorMode,
}

impl<'a, 'b> PromptLine<'a, 'b> {
    pub(crate) fn new(
        terminal: &'a mut Terminal,
        number_of_columns: u16,
        content: &'b str,
        selection: PromptInputSelection,
        focus: bool,
        mode: EditorMode,
    ) -> Self {
        Self {
            terminal,
            number_of_columns,
            content,
            selection,
            focus: PromptLineFocus::new(focus),
            mode,
        }
    }

    pub(crate) fn paint(&mut self) -> io::Result<()> {
        self.terminal.queue_add_command(MoveTo(0, 0))?;
        self.terminal
            .queue_add_command(Clear(ClearType::CurrentLine))?;
        let placeholder = PromptLinePlaceholder::new(self.mode);
        let display = placeholder.display_for(self.content, &self.focus);
        let display_content = display.text();

        // Leave one column of padding on both sides of the command line.
        let inner_width = self.number_of_columns.saturating_sub(2) as usize;
        let mut visible: String = display_content.chars().take(inner_width).collect();
        if visible.len() < inner_width {
            visible.push_str(&" ".repeat(inner_width - visible.len()));
        }
        let mut visible = format!(" {} ", visible);
        let target_width = self.number_of_columns as usize;
        if visible.len() < target_width {
            visible.push_str(&" ".repeat(target_width - visible.len()));
        } else if visible.len() > target_width {
            visible.truncate(target_width);
        }
        self.terminal
            .queue_add_command(SetBackgroundColor(Color::Rgb {
                r: 27,
                g: 27,
                b: 27,
            }))?;
        let highlight = PromptLineHighlight::new(self.selection.clone());
        let highlight_indices = highlight.visible_indices(display_content, inner_width);
        if highlight_indices.is_empty() {
            self.terminal
                .queue_add_command(SetForegroundColor(display.foreground()))?;
            self.terminal.queue_add_command(Print(visible))?;
        } else {
            PromptLineHighlight::queue(
                self.terminal,
                &visible,
                display.foreground(),
                highlight_indices,
            )?;
        }
        self.terminal.queue_add_command(ResetColor)?;
        Ok(())
    }
}
