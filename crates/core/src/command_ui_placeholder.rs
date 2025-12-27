use vimrust_protocol::CommandLineSelection;

pub struct CommandPlaceholder;

impl CommandPlaceholder {
    pub fn selection_for(&self, line: &str) -> CommandLineSelection {
        let start = match line.find('{') {
            Some(start) => start,
            None => return CommandLineSelection::None,
        };
        let tail = &line[start..];
        let end = match tail.find('}') {
            Some(offset) => start.saturating_add(offset).saturating_add(1),
            None => return CommandLineSelection::None,
        };
        if end <= start {
            return CommandLineSelection::None;
        }
        CommandLineSelection::range(start as u16, end as u16)
    }
}
