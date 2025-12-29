#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Normal,
    Edit,
    PromptCommand,
    PromptKeymap,
}

impl EditorMode {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Edit => "EDIT",
            EditorMode::PromptCommand => "PROMPT_COMMAND",
            EditorMode::PromptKeymap => "PROMPT_KEYMAP",
        }
    }
}
