use crate::{
    command_history_environment::CommandHistoryEnvironment,
    command_history_location::CommandHistoryLocation,
};
use vimrust_protocol::DocumentFile;

pub struct CommandHistoryStore {
    location: CommandHistoryLocation,
}

impl CommandHistoryStore {
    pub fn new() -> Self {
        let environment = CommandHistoryEnvironment;
        let location = environment.location();
        Self { location }
    }

    pub fn restore(&self, entries: &mut Vec<String>) {
        self.location.restore(entries);
    }

    pub fn append(&self, line: &str) {
        self.location.append(line);
    }

    pub fn file(&self) -> DocumentFile {
        self.location.file()
    }
}
