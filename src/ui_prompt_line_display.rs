use crossterm::style::Color;

use crate::ui_prompt_line_focus::PromptLineFocus;

pub(crate) struct PromptLineDisplay {
    text: String,
    color: Color,
}

impl PromptLineDisplay {
    pub(crate) fn placeholder(text: &str, focus: &PromptLineFocus) -> Self {
        Self {
            text: text.to_string(),
            color: focus.placeholder_foreground(),
        }
    }

    pub(crate) fn content(text: &str, focus: &PromptLineFocus) -> Self {
        Self {
            text: text.to_string(),
            color: focus.content_foreground(),
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn foreground(&self) -> Color {
        self.color
    }
}
