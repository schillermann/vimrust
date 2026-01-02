use std::{fs, io::Write, path::PathBuf};

use vimrust_protocol::FilePath;

pub struct CommandHistoryFile {
    directory: PathBuf,
    path: PathBuf,
}

impl CommandHistoryFile {
    pub fn new(directory: PathBuf, path: PathBuf) -> Self {
        Self { directory, path }
    }

    pub fn restore(&self, entries: &mut Vec<String>) {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    entries.push(trimmed.to_string());
                }
            }
            Err(_) => {}
        }
    }

    pub fn append(&self, line: &str) {
        if fs::create_dir_all(&self.directory).is_err() {
            return;
        }
        let mut file = match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(file) => file,
            Err(_) => return,
        };
        let _ = writeln!(file, "{}", line);
    }

    pub fn path(&self) -> FilePath {
        FilePath::Provided {
            path: self.path.to_string_lossy().to_string(),
        }
    }
}
