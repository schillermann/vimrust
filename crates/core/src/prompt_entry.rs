use vimrust_protocol::CommandListItemMode;

pub trait PromptEntry {
    fn label(&self) -> &str;
    fn detail(&self) -> &str;
    fn mode(&self) -> CommandListItemMode;
}
