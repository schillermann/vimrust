#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Normal,
    Edit,
    PromptCommand,
    PromptKeymap,
}
