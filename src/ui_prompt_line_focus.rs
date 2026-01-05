use crossterm::style::Color;

pub(crate) struct PromptLineFocus {
    on_prompt: bool,
}

impl PromptLineFocus {
    pub(crate) fn new(on_prompt: bool) -> Self {
        Self { on_prompt }
    }

    pub(crate) fn placeholder_foreground(&self) -> Color {
        if self.on_prompt {
            Color::White
        } else {
            Color::DarkGrey
        }
    }

    pub(crate) fn content_foreground(&self) -> Color {
        if self.on_prompt {
            Color::White
        } else {
            Color::DarkGrey
        }
    }
}
