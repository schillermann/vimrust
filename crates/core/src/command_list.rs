use crate::command_scope::CommandScope;
use crate::prompt_entry::PromptEntry;
use vimrust_protocol::PromptMode;

pub struct CommandEntry {
    name: &'static str,
    description: &'static str,
    mode: PromptMode,
}

pub struct CommandRegistry {
    commands: Vec<CommandEntry>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: vec![
                CommandEntry {
                    name: "s",
                    description: "Save the current buffer",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "save",
                    description: "Save the current buffer",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "q",
                    description: "Quit the editor",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "quit",
                    description: "Quit the editor",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "sq",
                    description: "Save and quit",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "o {filename}",
                    description: "Open a file",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "o",
                    description: "Reload the current file from disk",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "open {filename}",
                    description: "Open a file",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "open",
                    description: "Reload the current file from disk",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "case kebab",
                    description: "Convert selection to kebab-case",
                    mode: PromptMode::Visual,
                },
                CommandEntry {
                    name: "case camel",
                    description: "Convert selection to camelCase",
                    mode: PromptMode::Visual,
                },
            ],
        }
    }

    pub fn matching(&self, query: &str, scope: CommandScope) -> Vec<&dyn PromptEntry> {
        let normalized = CommandList::command_query_from_input(query);
        let mut matches = Vec::new();
        for entry in &self.commands {
            let allow = match scope {
                CommandScope::Normal => matches!(entry.mode, PromptMode::Command),
                CommandScope::Visual => matches!(entry.mode, PromptMode::Visual),
            };
            if !allow {
                continue;
            }
            let name = entry.label().to_lowercase();
            let desc = entry.detail().to_lowercase();
            if CommandList::fuzzy_match(&normalized, &name)
                || CommandList::fuzzy_match(&normalized, &desc)
            {
                matches.push(entry as &dyn PromptEntry);
            }
        }
        matches
    }
}

pub struct CommandList {
    registry: CommandRegistry,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl CommandList {
    pub fn new() -> Self {
        Self {
            registry: CommandRegistry::new(),
            selected_index: None,
            scroll_offset: 0,
        }
    }

    pub fn filter(&self, query: &str, scope: CommandScope) -> Vec<&dyn PromptEntry> {
        self.registry.matching(query, scope)
    }

    pub fn reset_selection(&mut self) {
        self.selected_index = None;
        self.scroll_offset = 0;
    }

    pub fn selection(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn scroll_position(&self) -> usize {
        self.scroll_offset
    }

    pub fn select_index(&mut self, new_index: usize) {
        self.selected_index = Some(new_index);
    }

    pub fn adjust_scroll_for_visible_rows(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.scroll_offset = 0;
            return;
        }
        if let Some(selected_index) = self.selected_index {
            if selected_index < self.scroll_offset {
                self.scroll_offset = selected_index;
            } else if selected_index >= self.scroll_offset.saturating_add(visible_rows) {
                self.scroll_offset = selected_index
                    .saturating_sub(visible_rows)
                    .saturating_add(1);
            }
        }
    }

    fn fuzzy_match(query: &str, candidate: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let mut query_chars = query.chars();
        let mut current = match query_chars.next() {
            Some(ch) => ch,
            None => return true,
        };

        for cand in candidate.chars() {
            if cand == current {
                if let Some(next) = query_chars.next() {
                    current = next;
                } else {
                    return true;
                }
            }
        }

        false
    }

    fn command_query_from_input(command_line: &str) -> String {
        let trimmed = command_line
            .trim_start_matches(':')
            .trim_start_matches(';')
            .trim();
        trimmed.to_lowercase()
    }
}

impl PromptEntry for CommandEntry {
    fn label(&self) -> &str {
        self.name
    }

    fn detail(&self) -> &str {
        self.description
    }

    fn mode(&self) -> PromptMode {
        self.mode.clone()
    }
}

impl Default for CommandList {
    fn default() -> Self {
        Self::new()
    }
}
