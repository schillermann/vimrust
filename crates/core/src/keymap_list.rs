use crate::prompt_entry::PromptEntry;
use vimrust_protocol::CommandListItemMode;

pub struct KeymapEntry {
    mode: CommandListItemMode,
    key: &'static str,
    description: &'static str,
}

pub struct KeymapRegistry {
    keymaps: Vec<KeymapEntry>,
}

impl KeymapRegistry {
    pub fn new() -> Self {
        Self {
            keymaps: vec![
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "q",
                    description: "Quit the editor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "e",
                    description: "Enter edit mode",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "s",
                    description: "Save the current buffer",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: ":",
                    description: "Open the command prompt",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: ";",
                    description: "Open the keymap prompt",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "h",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "j",
                    description: "Move cursor down",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "k",
                    description: "Move cursor up",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "l",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "PageUp",
                    description: "Move cursor one page up",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "PageDown",
                    description: "Move cursor one page down",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Normal,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Edit,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Edit,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::Edit,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Enter",
                    description: "Execute command",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Left",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Right",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Up",
                    description: "Move selection up",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptCommand,
                    key: "Down",
                    description: "Move selection down",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Left",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Right",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Up",
                    description: "Move selection up",
                },
                KeymapEntry {
                    mode: CommandListItemMode::PromptKeymap,
                    key: "Down",
                    description: "Move selection down",
                },
            ],
        }
    }

    pub fn matching(&self, query: &str) -> Vec<&dyn PromptEntry> {
        let normalized = KeymapList::query_from_input(query);
        let mut matches = Vec::new();
        for entry in &self.keymaps {
            let key_label = entry.label().to_string();
            let desc = entry.detail().to_lowercase();
            let mode_label = match entry.mode {
                CommandListItemMode::Command => "command",
                CommandListItemMode::Normal => "normal",
                CommandListItemMode::Edit => "edit",
                CommandListItemMode::PromptCommand => "prompt_command",
                CommandListItemMode::PromptKeymap => "prompt_keymap",
            };
            if KeymapList::fuzzy_match(&normalized, &key_label)
                || KeymapList::fuzzy_match(&normalized, &desc)
                || KeymapList::fuzzy_match(&normalized, mode_label)
            {
                matches.push(entry as &dyn PromptEntry);
            }
        }
        matches
    }
}

pub struct KeymapList {
    registry: KeymapRegistry,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl KeymapList {
    pub fn new() -> Self {
        Self {
            registry: KeymapRegistry::new(),
            selected_index: None,
            scroll_offset: 0,
        }
    }

    pub fn filter(&self, query: &str) -> Vec<&dyn PromptEntry> {
        self.registry.matching(query)
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

    fn query_from_input(prompt_line: &str) -> String {
        let trimmed = prompt_line
            .trim_start_matches(':')
            .trim_start_matches(';')
            .trim();
        trimmed.to_lowercase()
    }
}

impl PromptEntry for KeymapEntry {
    fn label(&self) -> &str {
        self.key
    }

    fn detail(&self) -> &str {
        self.description
    }

    fn mode(&self) -> CommandListItemMode {
        self.mode.clone()
    }
}

impl Default for KeymapList {
    fn default() -> Self {
        Self::new()
    }
}
