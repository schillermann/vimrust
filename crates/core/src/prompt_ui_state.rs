use crate::{
    command_completion::CommandCompletion, command_list::CommandList, command_scope::CommandScope,
    command_ui_placeholder::CommandPlaceholder, keymap_list::KeymapList, prompt_entry::PromptEntry,
    prompt_line::PromptLine, prompt_ui_snapshot::CommandUiSnapshot,
};
use vimrust_protocol::{
    CommandLineSelection, CommandListItemFrame, CommandUiAction, CommandUiFrame,
};

pub struct CommandUiState {
    prompt_line: PromptLine,
    command_list: CommandList,
    keymap_list: KeymapList,
    focus_on_list: bool,
    prompt_kind: PromptKind,
    command_scope: CommandScope,
}

impl CommandUiState {
    pub fn new() -> Self {
        Self {
            prompt_line: PromptLine::new(),
            command_list: CommandList::new(),
            keymap_list: KeymapList::new(),
            focus_on_list: false,
            prompt_kind: PromptKind::Command,
            command_scope: CommandScope::Normal,
        }
    }

    pub fn prompt_command(&mut self) {
        self.prompt_command_for(CommandScope::Normal);
    }

    pub fn prompt_command_for(&mut self, scope: CommandScope) {
        self.prompt_kind = PromptKind::Command;
        self.command_scope = scope;
        self.prompt_line.start_prompt(':');
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn prompt_keymap(&mut self) {
        self.prompt_kind = PromptKind::Keymap;
        self.prompt_line.start_prompt(';');
        self.keymap_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn line_overwrite(&mut self, new_content: String) {
        self.prompt_line.set_content(new_content);
        self.list_reset();
        self.focus_on_list = false;
    }

    pub fn command_text(&self) -> &str {
        self.prompt_line.text()
    }

    pub fn line_selection(&self) -> CommandLineSelection {
        self.prompt_line.selection()
    }

    pub fn clear(&mut self) {
        self.prompt_line.clear();
        self.list_reset();
        self.focus_on_list = false;
    }

    pub fn list_scroll_adjust(&mut self, visible_rows: usize) {
        self.list_scroll_reconcile(visible_rows);
    }

    pub fn apply_action(&mut self, action: CommandUiAction, list_rows: usize) {
        match action {
            CommandUiAction::StartPrompt => {
                self.prompt_command();
            }
            CommandUiAction::Clear => {
                self.clear();
            }
            CommandUiAction::InsertChar { ch } => {
                self.prompt_line.char_insert(ch);
                self.list_reset();
                self.focus_on_list = false;
            }
            CommandUiAction::Backspace => {
                self.prompt_line.backspace();
                self.list_reset();
                self.focus_on_list = false;
            }
            CommandUiAction::Delete => {
                self.prompt_line.delete();
                self.list_reset();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveLeft => {
                self.prompt_line.cursor_move_left();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveRight => {
                self.prompt_line.cursor_move_right();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveHome => {
                self.prompt_line.cursor_move_home();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveEnd => {
                self.prompt_line.cursor_move_end();
                self.focus_on_list = false;
            }
            CommandUiAction::Complete => {
                let completion = CommandCompletion::new(
                    self.prompt_line.text().to_string(),
                    self.prompt_line.cursor_column(),
                );
                completion.apply(&mut self.prompt_line);
                self.list_reset();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveSelectionUp | CommandUiAction::MoveSelectionDown => {
                let match_count = {
                    let matches = self.list_filter(self.prompt_line.text());
                    matches.len()
                };
                if match_count == 0 {
                    self.list_reset();
                    self.focus_on_list = false;
                    return;
                }

                self.focus_on_list = true;
                match self.list_selection() {
                    None => match action {
                        CommandUiAction::MoveSelectionDown => self.list_select_index(0),
                        CommandUiAction::MoveSelectionUp => {
                            self.list_select_index(match_count.saturating_sub(1));
                        }
                        _ => {}
                    },
                    Some(current_index) => {
                        let max_index = match_count.saturating_sub(1);
                        match action {
                            CommandUiAction::MoveSelectionUp if current_index > 0 => {
                                self.list_select_index(current_index.saturating_sub(1));
                            }
                            CommandUiAction::MoveSelectionDown if current_index < max_index => {
                                self.list_select_index(current_index.saturating_add(1));
                            }
                            _ => {}
                        }
                    }
                }
                self.list_scroll_adjust(list_rows);
            }
            CommandUiAction::SelectFromList => {
                if matches!(self.prompt_kind, PromptKind::Keymap) {
                    return;
                }
                let selected_label = {
                    let matches = self.list_filter(self.prompt_line.text());
                    if self.focus_on_list
                        && !matches.is_empty()
                        && let Some(selected) = self.list_selection()
                    {
                        let index = selected.min(matches.len().saturating_sub(1));
                        matches.get(index).map(|entry| entry.label().to_string())
                    } else {
                        None
                    }
                };
                if let Some(entry_label) = selected_label {
                    let line = format!(":{}", entry_label);
                    let placeholder = CommandPlaceholder;
                    let selection = placeholder.selection_for(&line);
                    self.prompt_line.set_content_with_selection(line, selection);
                    self.focus_on_list = false;

                    let updated_matches = self.list_filter(self.prompt_line.text());
                    let mut updated_index = None;
                    let mut idx = 0;
                    while idx < updated_matches.len() {
                        if updated_matches[idx].label() == entry_label {
                            updated_index = Some(idx);
                            break;
                        }
                        idx += 1;
                    }
                    if let Some(updated_index) = updated_index {
                        self.list_select_index(updated_index);
                        self.list_scroll_adjust(list_rows);
                    }
                    return;
                }
            }
        }
    }

    pub fn snapshot(&self) -> CommandUiSnapshot {
        CommandUiSnapshot::new(
            self.prompt_line.text().to_string(),
            self.prompt_line.cursor_column(),
            self.prompt_line.selection(),
            self.focus_on_list,
            self.list_selection(),
            self.list_scroll_position(),
        )
    }

    pub(crate) fn view(&self) -> CommandUiView<'_> {
        CommandUiView::new(self)
    }

    pub fn frame(&self) -> CommandUiFrame {
        let matches = self.list_filter(self.prompt_line.text());
        let selected_index = if let Some(idx) = self.list_selection() {
            if matches.is_empty() {
                None
            } else {
                Some(idx.min(matches.len().saturating_sub(1)))
            }
        } else {
            None
        };
        let mut list_items = Vec::with_capacity(matches.len());
        for entry in matches {
            list_items.push(CommandListItemFrame::new(
                entry.label().to_string(),
                entry.detail().to_string(),
                entry.mode(),
            ));
        }

        CommandUiFrame::new(
            self.prompt_line.text().to_string(),
            self.prompt_line.cursor_column(),
            self.prompt_line.selection(),
            self.focus_on_list,
            list_items,
            selected_index,
            self.list_scroll_position(),
        )
    }

    fn list_filter(&self, query: &str) -> Vec<&dyn PromptEntry> {
        match self.prompt_kind {
            PromptKind::Command => self.command_list.filter(query, self.command_scope),
            PromptKind::Keymap => self.keymap_list.filter(query),
        }
    }

    fn list_reset(&mut self) {
        match self.prompt_kind {
            PromptKind::Command => self.command_list.reset_selection(),
            PromptKind::Keymap => self.keymap_list.reset_selection(),
        }
    }

    fn list_selection(&self) -> Option<usize> {
        match self.prompt_kind {
            PromptKind::Command => self.command_list.selection(),
            PromptKind::Keymap => self.keymap_list.selection(),
        }
    }

    fn list_scroll_position(&self) -> usize {
        match self.prompt_kind {
            PromptKind::Command => self.command_list.scroll_position(),
            PromptKind::Keymap => self.keymap_list.scroll_position(),
        }
    }

    fn list_select_index(&mut self, new_index: usize) {
        match self.prompt_kind {
            PromptKind::Command => self.command_list.select_index(new_index),
            PromptKind::Keymap => self.keymap_list.select_index(new_index),
        }
    }

    fn list_scroll_reconcile(&mut self, visible_rows: usize) {
        match self.prompt_kind {
            PromptKind::Command => self
                .command_list
                .adjust_scroll_for_visible_rows(visible_rows),
            PromptKind::Keymap => self
                .keymap_list
                .adjust_scroll_for_visible_rows(visible_rows),
        }
    }
}

impl Default for CommandUiState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
enum PromptKind {
    Command,
    Keymap,
}

pub(crate) struct CommandUiView<'a> {
    pub(crate) text: &'a str,
    pub(crate) cursor: u16,
    pub(crate) selection: CommandLineSelection,
    pub(crate) focus_on_list: bool,
    pub(crate) selection_index: Option<usize>,
    pub(crate) scroll_offset: usize,
}

impl<'a> CommandUiView<'a> {
    pub(crate) fn new(state: &'a CommandUiState) -> Self {
        Self {
            text: state.prompt_line.text(),
            cursor: state.prompt_line.cursor_column(),
            selection: state.prompt_line.selection(),
            focus_on_list: state.focus_on_list,
            selection_index: state.list_selection(),
            scroll_offset: state.list_scroll_position(),
        }
    }
}
