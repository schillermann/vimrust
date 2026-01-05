use crate::{
    mode::EditorMode, ui_prompt_line_display::PromptLineDisplay,
    ui_prompt_line_focus::PromptLineFocus,
};

pub(crate) struct PromptLinePlaceholder {
    mode: EditorMode,
    empty_text: &'static str,
    visual_text: &'static str,
    edit_text: &'static str,
    command_text: &'static str,
    keymap_text: &'static str,
}

impl PromptLinePlaceholder {
    pub(crate) fn new(mode: EditorMode) -> Self {
        Self {
            mode,
            empty_text: "Press : for commands or ; for keymaps",
            visual_text: "Press : for visual commands or ; for keymaps",
            edit_text: "Press Esc to return to normal mode",
            command_text: ": type a command",
            keymap_text: "; filter keymaps",
        }
    }

    pub(crate) fn display_for(&self, content: &str, focus: &PromptLineFocus) -> PromptLineDisplay {
        if content.is_empty() {
            let message = match self.mode {
                EditorMode::Edit => self.edit_text,
                EditorMode::Visual => self.visual_text,
                _ => self.empty_text,
            };
            return PromptLineDisplay::placeholder(message, focus);
        }
        if content == ":" {
            return PromptLineDisplay::placeholder(self.command_text, focus);
        }
        if content == ";" {
            return PromptLineDisplay::placeholder(self.keymap_text, focus);
        }
        PromptLineDisplay::content(content, focus)
    }
}
