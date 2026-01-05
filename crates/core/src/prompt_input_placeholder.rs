use vimrust_protocol::PromptInputSelection;

pub struct PromptInputPlaceholder;

impl PromptInputPlaceholder {
    pub fn selection_for(&self, line: &str) -> PromptInputSelection {
        let start = match line.find('{') {
            Some(start) => start,
            None => return PromptInputSelection::None,
        };
        let tail = &line[start..];
        let end = match tail.find('}') {
            Some(offset) => start.saturating_add(offset).saturating_add(1),
            None => return PromptInputSelection::None,
        };
        if end <= start {
            return PromptInputSelection::None;
        }
        PromptInputSelection::range(start as u16, end as u16)
    }
}
