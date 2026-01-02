use crate::command_history_file::CommandHistoryFile;

pub enum CommandHistoryLocation {
    Missing,
    File { file: CommandHistoryFile },
}

impl CommandHistoryLocation {
    pub fn restore(&self, entries: &mut Vec<String>) {
        match self {
            CommandHistoryLocation::Missing => {}
            CommandHistoryLocation::File { file } => file.restore(entries),
        }
    }

    pub fn append(&self, line: &str) {
        match self {
            CommandHistoryLocation::Missing => {}
            CommandHistoryLocation::File { file } => file.append(line),
        }
    }
}
