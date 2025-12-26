use std::{fs, io};

use vimrust_protocol::FilePath;

pub struct File {
    path: FilePath,
    file_lines: Vec<String>,
    changed: bool,
    change_token: FileChangeToken,
}

impl File {
    pub fn new(file_path: FilePath) -> Self {
        Self {
            path: file_path,
            file_lines: Vec::new(),
            changed: false,
            change_token: FileChangeToken::new(),
        }
    }

    pub fn path(&self) -> FilePath {
        self.path.clone()
    }

    pub fn open(&mut self) -> io::Result<()> {
        let path = match &self.path {
            FilePath::Provided { path } => path.clone(),
            FilePath::Missing => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no file path set",
                ));
            }
        };
        let contents = fs::read_to_string(&path)?;
        let mut lines = Vec::new();
        for line in contents.lines() {
            lines.push(line.to_string());
        }
        self.file_lines = lines;

        if self.file_lines.is_empty() {
            self.file_lines.push(String::new());
        }

        self.changed = false;
        self.change_token = FileChangeToken::new();
        Ok(())
    }

    pub fn create(&mut self) {
        self.path = FilePath::Missing;
        self.file_lines.clear();
        self.file_lines.push(String::new());
        self.changed = false;
        self.change_token = FileChangeToken::new();
    }

    pub fn read(&mut self) -> io::Result<()> {
        match self.path {
            FilePath::Provided { .. } => self.open(),
            FilePath::Missing => {
                self.create();
                Ok(())
            }
        }
    }

    pub fn save(&mut self) -> io::Result<String> {
        let path = match &self.path {
            FilePath::Provided { path } => path.clone(),
            FilePath::Missing => String::from("untitled.txt"),
        };
        let contents = self.file_lines.join("\n");
        fs::write(&path, contents)?;
        if matches!(self.path, FilePath::Missing) {
            self.path = FilePath::Provided { path };
        }
        self.changed = false;
        self.change_token = FileChangeToken::new();
        Ok(String::from("saved"))
    }

    pub fn line_at(&self, index: usize) -> Option<&String> {
        self.file_lines.get(index)
    }

    pub fn line_count(&self) -> usize {
        self.file_lines.len()
    }

    pub fn line_at_mut(&mut self, index: usize) -> Option<&mut String> {
        self.file_lines.get_mut(index)
    }

    pub fn line_ensure(&mut self, index: usize) {
        if self.file_lines.len() <= index {
            self.file_lines
                .resize_with(index.saturating_add(1), String::new);
        }
    }

    pub fn line_remove(&mut self, index: usize) -> Option<String> {
        if index < self.file_lines.len() {
            Some(self.file_lines.remove(index))
        } else {
            None
        }
    }

    pub fn lines_snapshot(&self) -> Vec<String> {
        self.file_lines.clone()
    }

    pub fn lines_replace(&mut self, lines: Vec<String>) {
        self.file_lines = lines;
        if self.file_lines.is_empty() {
            self.file_lines.push(String::new());
        }
    }

    pub fn touch(&mut self) {
        self.changed = true;
        self.change_token = self.change_token.next();
    }

    pub fn change_state(&self) -> bool {
        self.changed
    }

    pub fn change_mark(&self) -> FileChangeToken {
        self.change_token
    }
}

// It’s an immutable marker of the file’s change state.
// Each edit (touch()) advances the token, while open/save/reset set it back to a new base.
// Snapshot queries compare the previous token to the current one to decide
// whether to emit a frame or update status, without returning booleans from mutating methods.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileChangeToken {
    value: u64,
}

impl FileChangeToken {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn next(&self) -> Self {
        Self {
            value: self.value.saturating_add(1),
        }
    }
}
