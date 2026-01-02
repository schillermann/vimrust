use crate::{
    command_history_environment::CommandHistoryEnvironment,
    command_history_location::CommandHistoryLocation,
};

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
}
