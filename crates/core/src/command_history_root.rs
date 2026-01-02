use std::path::PathBuf;

use crate::{
    command_history_directory::CommandHistoryDirectory,
    command_history_location::CommandHistoryLocation,
};

pub enum CommandHistoryRoot {
    Missing,
    Directory { directory: CommandHistoryDirectory },
}

impl CommandHistoryRoot {
    pub fn from(base: PathBuf) -> Self {
        Self::Directory {
            directory: CommandHistoryDirectory::new(base),
        }
    }

    pub fn location(self) -> CommandHistoryLocation {
        match self {
            CommandHistoryRoot::Missing => CommandHistoryLocation::Missing,
            CommandHistoryRoot::Directory { directory } => CommandHistoryLocation::File {
                file: directory.file(),
            },
        }
    }
}
