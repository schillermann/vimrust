use vimrust_protocol::PromptMode;

pub trait PromptEntry {
    fn label(&self) -> &str;
    fn detail(&self) -> &str;
    fn mode(&self) -> PromptMode;
}
