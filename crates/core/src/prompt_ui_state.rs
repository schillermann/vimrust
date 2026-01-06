use crate::{
    command_completion::CommandCompletion, command_history::CommandHistory,
    command_list::CommandList, command_scope::CommandScope, keymap_list::KeymapList,
    mode::EditorMode, prompt_entry::PromptEntry, prompt_input::PromptInput,
    prompt_input_placeholder::PromptInputPlaceholder, prompt_ui_snapshot::CommandUiSnapshot,
};
use vimrust_protocol::{
    FilePath, PromptInputSelection, PromptListItemFrame, PromptUiAction, PromptUiFrame,
    RequestEditorMode,
};

pub struct PromptUiState {
    line: PromptInput,
    line_focus: bool,
    list_command: CommandList,
    list_keymap: KeymapList,
    history_command: CommandHistory,
    prompt_kind: PromptKind,
    command_scope: CommandScope,
}

impl PromptUiState {
    pub fn new() -> Self {
        Self {
            line: PromptInput::new(),
            line_focus: true,
            list_command: CommandList::new(),
            list_keymap: KeymapList::new(),
            history_command: CommandHistory::new(),
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
        self.line.start_prompt(':');
        self.list_command.reset_selection();
        self.line_focus = true;
        self.history_command.reset_navigation();
    }

    pub fn prompt_keymap(&mut self) {
        self.prompt_kind = PromptKind::Keymap;
        self.line.start_prompt(';');
        self.list_keymap.reset_selection();
        self.line_focus = true;
        self.history_command.reset_navigation();
    }

    pub fn line_overwrite(&mut self, new_content: String) {
        self.line.set_content(new_content);
        self.list_reset();
        self.line_focus = true;
        self.history_command.reset_navigation();
        self.prefix_switch();
    }

    pub fn command_text(&self) -> &str {
        self.line.text()
    }

    pub fn history(&self) -> FilePath {
        self.history_command.file()
    }

    pub fn line_selection(&self) -> PromptInputSelection {
        self.line.selection()
    }

    pub fn clear(&mut self) {
        self.line.clear();
        self.list_reset();
        self.line_focus = true;
        self.history_command.reset_navigation();
    }

    pub fn prompt_mode_sync(&self, editor_mode: &mut EditorMode, path: &FilePath) {
        let requested = match self.prompt_kind {
            PromptKind::Command => RequestEditorMode::PromptCommand,
            PromptKind::Keymap => RequestEditorMode::PromptKeymap,
        };
        editor_mode.transition(requested, path);
    }

    pub fn list_scroll_adjust(&mut self, visible_rows: usize) {
        self.list_scroll_reconcile(visible_rows);
    }

    pub fn apply_action(&mut self, action: PromptUiAction, list_rows: usize) {
        match action {
            PromptUiAction::StartPrompt => {
                self.prompt_command();
            }
            PromptUiAction::Clear => {
                self.clear();
            }
            PromptUiAction::InsertChar { ch } => {
                self.line.char_insert(ch);
                self.list_reset();
                self.line_focus = true;
                self.history_command.reset_navigation();
                self.prefix_switch();
            }
            PromptUiAction::Backspace => {
                self.line.backspace();
                self.list_reset();
                self.line_focus = true;
                self.history_command.reset_navigation();
                self.prefix_switch();
            }
            PromptUiAction::Delete => {
                self.line.delete();
                self.list_reset();
                self.line_focus = true;
                self.history_command.reset_navigation();
                self.prefix_switch();
            }
            PromptUiAction::MoveLeft => {
                self.line.cursor_move_left();
                self.line_focus = true;
            }
            PromptUiAction::MoveRight => {
                self.line.cursor_move_right();
                self.line_focus = true;
            }
            PromptUiAction::MoveHome => {
                self.line.cursor_move_home();
                self.line_focus = true;
            }
            PromptUiAction::MoveEnd => {
                self.line.cursor_move_end();
                self.line_focus = true;
            }
            PromptUiAction::Complete => {
                let completion =
                    CommandCompletion::new(self.line.text().to_string(), self.line.cursor_column());
                completion.apply(&mut self.line);
                self.list_reset();
                self.line_focus = true;
                self.history_command.reset_navigation();
                self.prefix_switch();
            }
            PromptUiAction::FocusPrompt => {
                self.line_focus = true;
                self.list_selection_clear();
            }
            PromptUiAction::HistoryPrevious => {
                if matches!(self.prompt_kind, PromptKind::Command) {
                    if self.line_focus {
                        self.history_command.recall_previous(&mut self.line);
                        self.list_reset();
                        self.line_focus = true;
                        self.prefix_switch();
                    } else {
                        self.move_selection(PromptUiAction::MoveSelectionUp, list_rows);
                    }
                }
            }
            PromptUiAction::HistoryNext => {
                if matches!(self.prompt_kind, PromptKind::Command) {
                    if self.line_focus {
                        self.history_command.recall_next(&mut self.line);
                        self.list_reset();
                        self.line_focus = true;
                        self.prefix_switch();
                    } else {
                        self.move_selection(PromptUiAction::MoveSelectionDown, list_rows);
                    }
                }
            }
            PromptUiAction::MoveSelectionUp | PromptUiAction::MoveSelectionDown => {
                self.move_selection(action, list_rows);
            }
            PromptUiAction::SelectFromList => {
                if matches!(self.prompt_kind, PromptKind::Keymap) {
                    return;
                }
                let selected_label = {
                    let matches = self.list_filter(self.line.text());
                    if !self.line_focus
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
                    let placeholder = PromptInputPlaceholder;
                    let selection = placeholder.selection_for(&line);
                    self.line.set_content_with_selection(line, selection);
                    self.line_focus = true;
                    self.history_command.reset_navigation();
                    self.prefix_switch();

                    let updated_matches = self.list_filter(self.line.text());
                    let mut updated_index = None;
                    let mut idx = 0;
                    while idx < updated_matches.len() {
                        if updated_matches[idx].label() == entry_label {
                            updated_index = Some(idx);
                            break;
                        }
                        idx += 1;
                    }
                    if updated_index.is_some() {
                        self.list_scroll_adjust(list_rows);
                    }
                    self.list_selection_clear();
                    return;
                }
            }
        }
    }

    fn move_selection(&mut self, action: PromptUiAction, list_rows: usize) {
        let match_count = {
            let matches = self.list_filter(self.line.text());
            matches.len()
        };
        if match_count == 0 {
            self.list_reset();
            self.line_focus = true;
            return;
        }

        self.line_focus = false;
        match self.list_selection() {
            None => match action {
                PromptUiAction::MoveSelectionDown => self.list_select_index(0),
                PromptUiAction::MoveSelectionUp => {
                    self.list_select_index(match_count.saturating_sub(1));
                }
                _ => {}
            },
            Some(current_index) => {
                let max_index = match_count.saturating_sub(1);
                match action {
                    PromptUiAction::MoveSelectionUp if current_index > 0 => {
                        self.list_select_index(current_index.saturating_sub(1));
                    }
                    PromptUiAction::MoveSelectionDown if current_index < max_index => {
                        self.list_select_index(current_index.saturating_add(1));
                    }
                    _ => {}
                }
            }
        }
        self.list_scroll_adjust(list_rows);
    }

    fn prefix_switch(&mut self) {
        let mut chars = self.line.text().chars();
        match chars.next() {
            Some(';') => self.prompt_kind_change(PromptKind::Keymap),
            Some(':') => self.prompt_kind_change(PromptKind::Command),
            _ => {}
        }
    }

    fn prompt_kind_change(&mut self, target: PromptKind) {
        match (self.prompt_kind, target) {
            (PromptKind::Command, PromptKind::Command) => return,
            (PromptKind::Keymap, PromptKind::Keymap) => return,
            _ => {}
        }
        self.prompt_kind = target;
        self.list_reset();
        self.line_focus = true;
        self.history_command.reset_navigation();
    }

    pub fn remember_command(&mut self, line: &str) {
        if matches!(self.prompt_kind, PromptKind::Command) {
            self.history_command.record(line);
        }
    }

    pub fn snapshot(&self) -> CommandUiSnapshot {
        CommandUiSnapshot::new(
            self.line.text().to_string(),
            self.line.cursor_column(),
            self.line.selection(),
            self.line_focus,
            self.list_selection(),
            self.list_scroll_position(),
        )
    }

    pub(crate) fn view(&self) -> CommandUiView<'_> {
        CommandUiView::new(self)
    }

    pub fn frame(&self) -> PromptUiFrame {
        let matches = self.list_filter(self.line.text());
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
            list_items.push(PromptListItemFrame::new(
                entry.label().to_string(),
                entry.detail().to_string(),
                entry.mode(),
            ));
        }

        PromptUiFrame::new(
            self.line.text().to_string(),
            self.line_focus,
            self.line.cursor_column(),
            self.line.selection(),
            list_items,
            selected_index,
            self.list_scroll_position(),
        )
    }

    fn list_filter(&self, query: &str) -> Vec<&dyn PromptEntry> {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.filter(query, self.command_scope),
            PromptKind::Keymap => self.list_keymap.filter(query),
        }
    }

    fn list_reset(&mut self) {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.reset_selection(),
            PromptKind::Keymap => self.list_keymap.reset_selection(),
        }
    }

    fn list_selection(&self) -> Option<usize> {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.selection(),
            PromptKind::Keymap => self.list_keymap.selection(),
        }
    }

    fn list_selection_clear(&mut self) {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.selection_clear(),
            PromptKind::Keymap => self.list_keymap.selection_clear(),
        }
    }

    fn list_scroll_position(&self) -> usize {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.scroll_position(),
            PromptKind::Keymap => self.list_keymap.scroll_position(),
        }
    }

    fn list_select_index(&mut self, new_index: usize) {
        match self.prompt_kind {
            PromptKind::Command => self.list_command.select_index(new_index),
            PromptKind::Keymap => self.list_keymap.select_index(new_index),
        }
    }

    fn list_scroll_reconcile(&mut self, visible_rows: usize) {
        match self.prompt_kind {
            PromptKind::Command => self
                .list_command
                .adjust_scroll_for_visible_rows(visible_rows),
            PromptKind::Keymap => self
                .list_keymap
                .adjust_scroll_for_visible_rows(visible_rows),
        }
    }
}

impl Default for PromptUiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PromptUiState;
    use vimrust_protocol::{PromptListItemFrame, PromptUiAction};

    fn label_present(items: &[PromptListItemFrame], label: &str) -> bool {
        let mut idx = 0;
        while idx < items.len() {
            if items[idx].label() == label {
                return true;
            }
            idx = idx.saturating_add(1);
        }
        false
    }

    #[test]
    fn prompt_switches_to_keymap_on_semicolon_prefix() {
        let mut prompt = PromptUiState::new();
        prompt.prompt_command();
        prompt.apply_action(PromptUiAction::Backspace, 10);
        prompt.apply_action(PromptUiAction::InsertChar { ch: ';' }, 10);

        let frame = prompt.frame();
        assert!(label_present(frame.command_items(), ":"));
        assert!(!label_present(frame.command_items(), "o {path[:line[:column]]}"));
    }

    #[test]
    fn prompt_switches_to_command_on_colon_prefix() {
        let mut prompt = PromptUiState::new();
        prompt.prompt_keymap();
        prompt.apply_action(PromptUiAction::Backspace, 10);
        prompt.apply_action(PromptUiAction::InsertChar { ch: ':' }, 10);

        let frame = prompt.frame();
        assert!(label_present(frame.command_items(), "o {path[:line[:column]]}"));
        assert!(!label_present(frame.command_items(), "Ctrl+Down"));
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
    pub(crate) selection: PromptInputSelection,
    pub(crate) line_focus: bool,
    pub(crate) selection_index: Option<usize>,
    pub(crate) scroll_offset: usize,
}

impl<'a> CommandUiView<'a> {
    pub(crate) fn new(state: &'a PromptUiState) -> Self {
        Self {
            text: state.line.text(),
            cursor: state.line.cursor_column(),
            selection: state.line.selection(),
            line_focus: state.line_focus,
            selection_index: state.list_selection(),
            scroll_offset: state.list_scroll_position(),
        }
    }
}
