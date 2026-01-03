use vimrust_protocol::CommandSelection;

pub struct CommandPlaceholder;

impl CommandPlaceholder {
    pub fn selection_for(&self, line: &str) -> CommandSelection {
        let start = match line.find('{') {
            Some(start) => start,
            None => return CommandSelection::None,
        };
        let tail = &line[start..];
        let end = match tail.find('}') {
            Some(offset) => start.saturating_add(offset).saturating_add(1),
            None => return CommandSelection::None,
        };
        if end <= start {
            return CommandSelection::None;
        }
        CommandSelection::range(start as u16, end as u16)
    }
}
