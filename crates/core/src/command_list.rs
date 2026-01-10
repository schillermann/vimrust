use crate::command_scope::CommandScope;
use crate::prompt_entry::PromptEntry;
use std::rc::Rc;
use vimrust_protocol::{PromptListSelection, PromptMode};

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
                    name: "o {path[:line[:column]]}",
                    description: "Open a file",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "o",
                    description: "Reload the current file from disk",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "open {path[:line[:column]]}",
                    description: "Open a file",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "open",
                    description: "Reload the current file from disk",
                    mode: PromptMode::Command,
                },
                CommandEntry {
                    name: "history",
                    description: "Open command prompt history",
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
                CommandEntry {
                    name: "case snake",
                    description: "Convert selection to snake_case",
                    mode: PromptMode::Visual,
                },
                CommandEntry {
                    name: "case screaming",
                    description: "Convert selection to SCREAMING_SNAKE_CASE",
                    mode: PromptMode::Visual,
                },
                CommandEntry {
                    name: "case pascal",
                    description: "Convert selection to PascalCase",
                    mode: PromptMode::Visual,
                },
                CommandEntry {
                    name: "case train",
                    description: "Convert selection to Train-Case",
                    mode: PromptMode::Visual,
                },
                CommandEntry {
                    name: "case flat",
                    description: "Convert selection to flatcase",
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
    registry: Rc<CommandRegistry>,
    selection: CommandListSelection,
}

impl CommandList {
    pub fn new() -> Self {
        Self {
            registry: Rc::new(CommandRegistry::new()),
            selection: CommandListSelection::new(),
        }
    }

    pub fn filter(&self, query: &str, scope: CommandScope) -> Vec<&dyn PromptEntry> {
        self.registry.matching(query, scope)
    }

    pub fn reset_selection(&self) -> Self {
        Self {
            registry: Rc::clone(&self.registry),
            selection: self.selection.reset(),
        }
    }

    pub fn selection_clear(&self) -> Self {
        Self {
            registry: Rc::clone(&self.registry),
            selection: self.selection.cleared(),
        }
    }

    pub fn selection(&self) -> PromptListSelection {
        self.selection.selection()
    }

    pub fn scroll_position(&self) -> usize {
        self.selection.scroll_position()
    }

    pub fn select_index(&self, new_index: usize) -> Self {
        Self {
            registry: Rc::clone(&self.registry),
            selection: self.selection.selected(new_index),
        }
    }

    pub fn adjust_scroll_for_visible_rows(&self, visible_rows: usize) -> Self {
        Self {
            registry: Rc::clone(&self.registry),
            selection: self.selection.adjusted(visible_rows),
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

#[derive(Clone)]
struct CommandListSelection {
    selected_index: PromptListSelection,
    scroll_offset: usize,
}

impl CommandListSelection {
    fn new() -> Self {
        Self {
            selected_index: PromptListSelection::empty(),
            scroll_offset: 0,
        }
    }

    fn reset(&self) -> Self {
        Self {
            selected_index: PromptListSelection::empty(),
            scroll_offset: 0,
        }
    }

    fn cleared(&self) -> Self {
        Self {
            selected_index: PromptListSelection::empty(),
            scroll_offset: self.scroll_offset,
        }
    }

    fn selected(&self, new_index: usize) -> Self {
        Self {
            selected_index: PromptListSelection::at(new_index),
            scroll_offset: self.scroll_offset,
        }
    }

    fn adjusted(&self, visible_rows: usize) -> Self {
        if visible_rows == 0 {
            return Self {
                selected_index: self.selected_index.clone(),
                scroll_offset: 0,
            };
        }
        let selected_index = self.selected_index.index();
        let empty_index = PromptListSelection::empty().index();
        if selected_index != empty_index {
            if selected_index < self.scroll_offset {
                return Self {
                    selected_index: self.selected_index.clone(),
                    scroll_offset: selected_index,
                };
            }
            if selected_index >= self.scroll_offset.saturating_add(visible_rows) {
                return Self {
                    selected_index: self.selected_index.clone(),
                    scroll_offset: selected_index
                        .saturating_sub(visible_rows)
                        .saturating_add(1),
                };
            }
        }
        self.clone()
    }

    fn selection(&self) -> PromptListSelection {
        self.selected_index.clone()
    }

    fn scroll_position(&self) -> usize {
        self.scroll_offset
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
