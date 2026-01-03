use crate::prompt_entry::PromptEntry;
use vimrust_protocol::PromptMode;

pub struct KeymapEntry {
    mode: PromptMode,
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
                    mode: PromptMode::Normal,
                    key: "q",
                    description: "Quit the editor",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "e",
                    description: "Enter edit mode",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "s",
                    description: "Save the current buffer",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: ":",
                    description: "Open the command prompt",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "v",
                    description: "Enter visual mode",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: ";",
                    description: "Open the keymap prompt",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "h",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "j",
                    description: "Move cursor down",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "k",
                    description: "Move cursor up",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "l",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "PageUp",
                    description: "Move cursor one page up",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "PageDown",
                    description: "Move cursor one page down",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: PromptMode::Normal,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: PromptMode::Edit,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: PromptMode::Edit,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: PromptMode::Edit,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: PromptMode::Edit,
                    key: "Enter",
                    description: "Insert line break",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: ":",
                    description: "Open the command prompt",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "h",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "j",
                    description: "Move cursor down",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "k",
                    description: "Move cursor up",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "l",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "PageUp",
                    description: "Move cursor one page up",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "PageDown",
                    description: "Move cursor one page down",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: PromptMode::Visual,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Enter",
                    description: "Execute command",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Left",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Right",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Tab",
                    description: "Complete file name in command prompt",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Ctrl+Up",
                    description: "Move focus to command line",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Ctrl+Down",
                    description: "Move focus to command list",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Up",
                    description: "Recall previous command from history",
                },
                KeymapEntry {
                    mode: PromptMode::PromptCommand,
                    key: "Down",
                    description: "Recall next command from history",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Esc",
                    description: "Return to normal mode",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Backspace",
                    description: "Delete character before cursor",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Delete",
                    description: "Delete character under cursor",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Left",
                    description: "Move cursor left",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Right",
                    description: "Move cursor right",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Home",
                    description: "Move cursor to line start",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "End",
                    description: "Move cursor to line end",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Ctrl+Up",
                    description: "Move focus to keymap input",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Ctrl+Down",
                    description: "Move focus to keymap list",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
                    key: "Up",
                    description: "Move selection up",
                },
                KeymapEntry {
                    mode: PromptMode::PromptKeymap,
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
                PromptMode::Command => "command",
                PromptMode::Normal => "normal",
                PromptMode::Edit => "edit",
                PromptMode::Visual => "visual",
                PromptMode::PromptCommand => "prompt_command",
                PromptMode::PromptKeymap => "prompt_keymap",
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

    pub fn selection_clear(&mut self) {
        self.selected_index = None;
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

    fn query_from_input(prompt_input: &str) -> String {
        let trimmed = prompt_input
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

    fn mode(&self) -> PromptMode {
        self.mode.clone()
    }
}

impl Default for KeymapList {
    fn default() -> Self {
        Self::new()
    }
}
