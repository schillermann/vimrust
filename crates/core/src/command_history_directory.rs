use std::path::PathBuf;

use crate::command_history_file::CommandHistoryFile;

pub struct CommandHistoryDirectory {
    path: PathBuf,
}

impl CommandHistoryDirectory {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn file(self) -> CommandHistoryFile {
        let directory = self.path.join("vimrust");
        let path = directory.join("history");
        CommandHistoryFile::new(directory, path)
    }
}
