use std::{fs, io};

use vimrust_protocol::{DocumentFile, StatusMessage};

#[derive(Clone)]
pub struct File {
    path: DocumentFile,
    file_lines: Vec<String>,
    changed: bool,
    change_token: FileChangeToken,
}

impl File {
    pub fn new(file_path: DocumentFile) -> Self {
        Self {
            path: file_path,
            file_lines: Vec::new(),
            changed: false,
            change_token: FileChangeToken::new(),
        }
    }

    pub fn path(&self) -> DocumentFile {
        self.path.clone()
    }

    pub fn open(&mut self) -> io::Result<()> {
        let path = self.path.open_path()?;
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
        self.path = DocumentFile {
            path: String::new(),
        };
        self.file_lines.clear();
        self.file_lines.push(String::new());
        self.changed = false;
        self.change_token = FileChangeToken::new();
    }

    pub fn read(&mut self) -> io::Result<()> {
        if self.path.path.is_empty() {
            self.create();
            Ok(())
        } else {
            self.open()
        }
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.save_path();
        let contents = self.file_lines.join("\n");
        fs::write(&path, contents)?;
        if self.path.path.is_empty() {
            self.path = DocumentFile { path };
        }
        self.changed = false;
        self.change_token = FileChangeToken::new();
        Ok(())
    }

    pub fn message_lock(&self) -> StatusMessage {
        let readonly = match self.path.metadata_path() {
            Ok(path) => match fs::metadata(path) {
                Ok(metadata) => metadata.permissions().readonly(),
                Err(_) => false,
            },
            Err(_) => false,
        };
        if readonly {
            StatusMessage::Text {
                text: String::from("locked"),
            }
        } else {
            StatusMessage::Empty
        }
    }

    pub fn line_at(&self, index: usize) -> &str {
        self.file_lines[index].as_str()
    }

    pub fn line_count(&self) -> usize {
        self.file_lines.len()
    }

    pub fn line_total(&self) -> u16 {
        self.file_lines.len().min(u16::MAX as usize) as u16
    }

    pub fn line_at_mut(&mut self, index: usize) -> &mut String {
        self.file_lines.get_mut(index).expect("line index in range")
    }

    pub fn line_ensure(&mut self, index: usize) {
        if self.file_lines.len() <= index {
            self.file_lines
                .resize_with(index.saturating_add(1), String::new);
        }
    }

    pub fn line_remove(&mut self, index: usize) {
        if index < self.file_lines.len() {
            self.file_lines.remove(index);
        }
    }

    pub fn line_insert(&mut self, index: usize, line: String) {
        if index >= self.file_lines.len() {
            self.file_lines.push(line);
        } else {
            self.file_lines.insert(index, line);
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
