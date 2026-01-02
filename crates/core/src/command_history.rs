use crate::{command_history_store::CommandHistoryStore, prompt_line::PromptLine};

pub struct CommandHistory {
    entries: Vec<String>,
    cursor: CommandHistoryCursor,
    draft: CommandHistoryDraft,
    store: CommandHistoryStore,
}

impl CommandHistory {
    pub fn new() -> Self {
        let store = CommandHistoryStore::new();
        let mut history = Self {
            entries: Vec::new(),
            cursor: CommandHistoryCursor::Tail,
            draft: CommandHistoryDraft::Empty,
            store,
        };
        history.store.restore(&mut history.entries);
        history
    }

    pub fn record(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == ":" {
            return;
        }
        self.entries.push(line.to_string());
        self.store.append(line);
        self.cursor = CommandHistoryCursor::Tail;
        self.draft = CommandHistoryDraft::Empty;
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = CommandHistoryCursor::Tail;
        self.draft = CommandHistoryDraft::Empty;
    }

    pub fn recall_previous(&mut self, prompt_line: &mut PromptLine) {
        if self.entries.is_empty() {
            return;
        }
        match self.cursor {
            CommandHistoryCursor::Tail => {
                let draft = prompt_line.text().to_string();
                self.draft = CommandHistoryDraft::Stored { line: draft };
                let index = self.entries.len().saturating_sub(1);
                self.cursor = CommandHistoryCursor::At { index };
                self.apply_index(prompt_line, index);
            }
            CommandHistoryCursor::At { index } => {
                if index == 0 {
                    self.apply_index(prompt_line, index);
                    return;
                }
                let next_index = index.saturating_sub(1);
                self.cursor = CommandHistoryCursor::At { index: next_index };
                self.apply_index(prompt_line, next_index);
            }
        }
    }

    pub fn recall_next(&mut self, prompt_line: &mut PromptLine) {
        match self.cursor {
            CommandHistoryCursor::Tail => {}
            CommandHistoryCursor::At { index } => {
                let next_index = index.saturating_add(1);
                if next_index < self.entries.len() {
                    self.cursor = CommandHistoryCursor::At { index: next_index };
                    self.apply_index(prompt_line, next_index);
                    return;
                }
                self.cursor = CommandHistoryCursor::Tail;
                self.restore_draft(prompt_line);
            }
        }
    }

    fn apply_index(&self, prompt_line: &mut PromptLine, index: usize) {
        if index < self.entries.len() {
            prompt_line.set_content(self.entries[index].clone());
        }
    }

    fn restore_draft(&mut self, prompt_line: &mut PromptLine) {
        match &self.draft {
            CommandHistoryDraft::Stored { line } => {
                prompt_line.set_content(line.clone());
            }
            CommandHistoryDraft::Empty => {}
        }
        self.draft = CommandHistoryDraft::Empty;
    }
}

enum CommandHistoryCursor {
    Tail,
    At { index: usize },
}

enum CommandHistoryDraft {
    Empty,
    Stored { line: String },
}
