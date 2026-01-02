use crate::prompt_line::PromptLine;

pub struct CommandHistory {
    entries: Vec<String>,
    cursor: HistoryCursor,
    draft: HistoryDraft,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: HistoryCursor::Tail,
            draft: HistoryDraft::Empty,
        }
    }

    pub fn record(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == ":" {
            return;
        }
        self.entries.push(line.to_string());
        self.cursor = HistoryCursor::Tail;
        self.draft = HistoryDraft::Empty;
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = HistoryCursor::Tail;
        self.draft = HistoryDraft::Empty;
    }

    pub fn recall_previous(&mut self, prompt_line: &mut PromptLine) {
        if self.entries.is_empty() {
            return;
        }
        match self.cursor {
            HistoryCursor::Tail => {
                let draft = prompt_line.text().to_string();
                self.draft = HistoryDraft::Stored { line: draft };
                let index = self.entries.len().saturating_sub(1);
                self.cursor = HistoryCursor::At { index };
                self.apply_index(prompt_line, index);
            }
            HistoryCursor::At { index } => {
                if index == 0 {
                    self.apply_index(prompt_line, index);
                    return;
                }
                let next_index = index.saturating_sub(1);
                self.cursor = HistoryCursor::At { index: next_index };
                self.apply_index(prompt_line, next_index);
            }
        }
    }

    pub fn recall_next(&mut self, prompt_line: &mut PromptLine) {
        match self.cursor {
            HistoryCursor::Tail => {}
            HistoryCursor::At { index } => {
                let next_index = index.saturating_add(1);
                if next_index < self.entries.len() {
                    self.cursor = HistoryCursor::At { index: next_index };
                    self.apply_index(prompt_line, next_index);
                    return;
                }
                self.cursor = HistoryCursor::Tail;
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
            HistoryDraft::Stored { line } => {
                prompt_line.set_content(line.clone());
            }
            HistoryDraft::Empty => {}
        }
        self.draft = HistoryDraft::Empty;
    }
}

enum HistoryCursor {
    Tail,
    At { index: usize },
}

enum HistoryDraft {
    Empty,
    Stored { line: String },
}
