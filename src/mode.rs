#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Normal,
    Edit,
    Command,
}

impl EditorMode {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Edit => "EDIT",
            EditorMode::Command => "COMMAND",
        }
    }
}
