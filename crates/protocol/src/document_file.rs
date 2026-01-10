use std::fmt;
use std::io;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct DocumentFile {
    pub path: String,
}

impl DocumentFile {
    pub fn open_path(&self) -> io::Result<String> {
        if self.path.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no file path set",
            ))
        } else {
            Ok(self.path.clone())
        }
    }

    pub fn save_path(&self) -> String {
        if self.path.is_empty() {
            String::from("untitled.txt")
        } else {
            self.path.clone()
        }
    }

    pub fn metadata_path(&self) -> io::Result<String> {
        if self.path.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no file path set",
            ))
        } else {
            Ok(self.path.clone())
        }
    }

}

impl fmt::Display for DocumentFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "[No Filename]")
        } else {
            write!(f, "{}", self.path)
        }
    }
}
