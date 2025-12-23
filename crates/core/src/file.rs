use std::{fs, io};

use vimrust_protocol::FilePath;

pub struct File {
    path: FilePath,
    file_lines: Vec<String>,
    changed: bool,
}

impl File {
    pub fn new(file_path: FilePath) -> Self {
        Self {
            path: file_path,
            file_lines: Vec::new(),
            changed: false,
        }
    }

    pub fn location(&self) -> FilePath {
        self.path.clone()
    }

    pub fn open(&mut self) -> io::Result<()> {
        let path = match &self.path {
            FilePath::Provided { path } => path.clone(),
            FilePath::Missing => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no file path set",
                ))
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
        Ok(())
    }

    pub fn create(&mut self) {
        self.path = FilePath::Missing;
        self.file_lines.clear();
        self.file_lines.push(String::new());
        self.changed = false;
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
        Ok(String::from("saved"))
    }

    pub fn line(&self, index: usize) -> Option<&String> {
        self.file_lines.get(index)
    }

    pub fn line_total(&self) -> usize {
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

    pub fn lines_clone(&self) -> Vec<String> {
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
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
