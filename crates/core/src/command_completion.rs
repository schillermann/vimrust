use std::{fs, path::PathBuf};

use crate::prompt_input::PromptInput;

pub struct CommandCompletion {
    line: String,
    cursor: u16,
}

impl CommandCompletion {
    pub fn new(line: String, cursor: u16) -> Self {
        Self { line, cursor }
    }

    pub fn apply(self, prompt_input: &mut PromptInput) {
        let target = CompletionTarget::new(self.line, self.cursor);
        target.apply(prompt_input);
    }
}

enum CompletionTarget {
    Open(OpenPathTarget),
    Skip,
}

impl CompletionTarget {
    fn new(line: String, cursor: u16) -> Self {
        let cursor_index = cursor as usize;
        if cursor_index != line.len() {
            return CompletionTarget::Skip;
        }
        let bytes = line.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx = idx.saturating_add(1);
        }
        if idx >= bytes.len() || bytes[idx] != b':' {
            return CompletionTarget::Skip;
        }
        idx = idx.saturating_add(1);
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx = idx.saturating_add(1);
        }
        let name_start = idx;
        while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
            idx = idx.saturating_add(1);
        }
        if name_start >= idx {
            return CompletionTarget::Skip;
        }
        let name = &line[name_start..idx];
        if name != "o" && name != "open" {
            return CompletionTarget::Skip;
        }
        let mut path_start = idx;
        while path_start < bytes.len() && bytes[path_start].is_ascii_whitespace() {
            path_start = path_start.saturating_add(1);
        }
        let prefix = line[..path_start].to_string();
        let path = line[path_start..].to_string();
        CompletionTarget::Open(OpenPathTarget::new(prefix, path))
    }

    fn apply(self, prompt_input: &mut PromptInput) {
        match self {
            CompletionTarget::Open(target) => target.apply(prompt_input),
            CompletionTarget::Skip => {}
        }
    }
}

struct OpenPathTarget {
    prefix: String,
    path: String,
}

impl OpenPathTarget {
    fn new(prefix: String, path: String) -> Self {
        Self { prefix, path }
    }

    fn apply(self, prompt_input: &mut PromptInput) {
        let working_dir = WorkingDirectory::new();
        let request = PathCompletionRequest::new(working_dir.path(), self.path);
        let completion = request.resolve();
        completion.apply(prompt_input, self.prefix);
    }
}

struct WorkingDirectory;

impl WorkingDirectory {
    fn new() -> Self {
        Self
    }

    fn path(&self) -> PathBuf {
        match std::env::current_dir() {
            Ok(path) => path,
            Err(_) => PathBuf::from("."),
        }
    }
}

struct PathCompletionRequest {
    base_dir: PathBuf,
    leader: String,
    prefix: String,
}

impl PathCompletionRequest {
    fn new(working_dir: PathBuf, path: String) -> Self {
        let path = if path.contains('{') || path.contains('}') {
            String::new()
        } else {
            path
        };
        let is_absolute = path.starts_with('/');
        let path_value = path.clone();
        let mut last_slash = SlashPosition::Missing;
        for (idx, ch) in path.char_indices() {
            if ch == '/' {
                last_slash = SlashPosition::Found(idx);
            }
        }
        let (leader, prefix) = match last_slash {
            SlashPosition::Missing => (String::new(), path_value),
            SlashPosition::Found(idx) => {
                let leader = path_value[..=idx].to_string();
                let prefix = path_value[idx.saturating_add(1)..].to_string();
                (leader, prefix)
            }
        };
        let base_dir = if is_absolute {
            if leader.is_empty() {
                PathBuf::from("/")
            } else {
                PathBuf::from(leader.as_str())
            }
        } else if leader.is_empty() {
            working_dir
        } else {
            let mut base = working_dir;
            base.push(leader.as_str());
            base
        };
        Self {
            base_dir,
            leader,
            prefix,
        }
    }

    fn resolve(self) -> PathCompletion {
        let listing = DirectoryListing::new(self.base_dir);
        let completion = listing.completion_for(self.prefix.as_str());
        completion.with_leader(self.leader)
    }
}

enum SlashPosition {
    Found(usize),
    Missing,
}

struct DirectoryListing {
    entries: Vec<String>,
}

impl DirectoryListing {
    fn new(base_dir: PathBuf) -> Self {
        let mut entries = Vec::new();
        let read_dir = fs::read_dir(base_dir);
        if let Ok(read_dir) = read_dir {
            for item in read_dir {
                if let Ok(entry) = item {
                    let mut label = entry.file_name().to_string_lossy().to_string();
                    if label.is_empty() {
                        continue;
                    }
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() && !label.ends_with('/') {
                            label.push('/');
                        }
                    }
                    entries.push(label);
                }
            }
        }
        entries.sort();
        Self { entries }
    }

    fn completion_for(&self, prefix: &str) -> CompletionCandidate {
        let mut matches = Vec::new();
        for entry in &self.entries {
            if entry.starts_with(prefix) {
                matches.push(entry.clone());
            }
        }
        if matches.is_empty() {
            return CompletionCandidate::NoChange;
        }
        let mut common = CommonPrefix::new(matches[0].clone());
        let mut idx = 1;
        while idx < matches.len() {
            common.refine(matches[idx].as_str());
            idx = idx.saturating_add(1);
        }
        let value = common.value();
        if value == prefix {
            CompletionCandidate::NoChange
        } else {
            CompletionCandidate::Replace { value }
        }
    }
}

struct CommonPrefix {
    value: String,
}

impl CommonPrefix {
    fn new(value: String) -> Self {
        Self { value }
    }

    fn refine(&mut self, candidate: &str) {
        let bytes = self.value.as_bytes();
        let cand_bytes = candidate.as_bytes();
        let mut idx = 0;
        let max = bytes.len().min(cand_bytes.len());
        while idx < max && bytes[idx] == cand_bytes[idx] {
            idx = idx.saturating_add(1);
        }
        self.value.truncate(idx);
    }

    fn value(self) -> String {
        self.value
    }
}

enum CompletionCandidate {
    Replace { value: String },
    NoChange,
}

impl CompletionCandidate {
    fn with_leader(self, leader: String) -> PathCompletion {
        match self {
            CompletionCandidate::Replace { value } => PathCompletion::Replace {
                value: format!("{}{}", leader, value),
            },
            CompletionCandidate::NoChange => PathCompletion::NoChange,
        }
    }
}

enum PathCompletion {
    Replace { value: String },
    NoChange,
}

impl PathCompletion {
    fn apply(self, prompt_input: &mut PromptInput, prefix: String) {
        match self {
            PathCompletion::Replace { value } => {
                prompt_input.set_content(format!("{}{}", prefix, value));
            }
            PathCompletion::NoChange => {}
        }
    }
}
