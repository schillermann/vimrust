use std::{fs, io};

pub struct File {
    path: Option<String>,
    pub file_lines: Vec<String>,
    changed: bool,
}

impl File {
    pub fn new(file_path: Option<String>) -> Self {
        Self {
            path: file_path,
            file_lines: Vec::new(),
            changed: false,
        }
    }

    pub fn path(&self) -> Option<&String> {
        self.path.as_ref()
    }

    pub fn open(&mut self) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file path set"))?;
        let contents = fs::read_to_string(&path)?;
        self.file_lines = contents.lines().map(|line| line.to_string()).collect();

        if self.file_lines.is_empty() {
            self.file_lines.push(String::new());
        }

        self.changed = false;
        Ok(())
    }

    pub fn create(&mut self) {
        self.path = None;
        self.file_lines.clear();
        self.file_lines.push(String::new());
        self.changed = false;
    }

    pub fn read(&mut self) -> io::Result<()> {
        match self.path {
            Some(_) => self.open(),
            None => {
                self.create();
                Ok(())
            }
        }
    }

    pub fn save(&mut self) -> io::Result<String> {
        let path = self
            .path
            .get_or_insert_with(|| String::from("untitled.txt"))
            .clone();
        let contents = self.file_lines.join("\n");
        fs::write(&path, contents)?;
        self.changed = false;
        Ok(String::from("saved"))
    }

    pub fn line(&self, index: usize) -> Option<&String> {
        self.file_lines.get(index)
    }

    pub fn len(&self) -> usize {
        self.file_lines.len()
    }

    pub fn touch(&mut self) {
        self.changed = true;
    }

    pub fn changed(&self) -> bool {
        self.changed
    }
}
