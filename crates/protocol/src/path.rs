use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilePath {
    Missing,
    Provided { path: String },
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilePath::Missing => write!(f, "[No Filename]"),
            FilePath::Provided { path } => write!(f, "{}", path),
        }
    }
}
