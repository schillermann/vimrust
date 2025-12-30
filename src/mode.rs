#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Normal,
    Edit,
    PromptCommand,
    PromptKeymap,
}

impl EditorMode {
    pub(crate) fn append_to(&self, target: &mut String) {
        match self {
            EditorMode::Normal => target.push_str("NORMAL"),
            EditorMode::Edit => target.push_str("EDIT"),
            EditorMode::PromptCommand => target.push_str("PROMPT_COMMAND"),
            EditorMode::PromptKeymap => target.push_str("PROMPT_KEYMAP"),
        }
    }
}
