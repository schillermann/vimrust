use crate::{command_line::CommandLine, command_list::CommandList, frame_signal::FrameSignal};
use vimrust_protocol::{CommandListItemFrame, CommandUiAction, CommandUiFrame};

pub struct CommandUiState {
    command_line: CommandLine,
    command_list: CommandList,
    focus_on_list: bool,
}

pub struct CommandUiSnapshot {
    command_text: String,
    cursor_column: u16,
    focus_on_list: bool,
    selection: Option<usize>,
    scroll_offset: usize,
}

impl CommandUiSnapshot {
    pub fn frame_signal(&self, state: &CommandUiState) -> FrameSignal {
        let same_text = self.command_text == state.command_line.text();
        let same_cursor = self.cursor_column == state.command_line.cursor_column();
        let same_focus = self.focus_on_list == state.focus_on_list;
        let same_selection = self.selection == state.command_list.selection();
        let same_scroll = self.scroll_offset == state.command_list.scroll_position();
        if same_text && same_cursor && same_focus && same_selection && same_scroll {
            FrameSignal::Skip
        } else {
            FrameSignal::Frame
        }
    }
}

impl CommandUiState {
    pub fn new() -> Self {
        Self {
            command_line: CommandLine::new(),
            command_list: CommandList::new(),
            focus_on_list: false,
        }
    }

    pub fn start_prompt(&mut self) {
        self.command_line.start_prompt();
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn line_overwrite(&mut self, new_content: String) {
        self.command_line.set_content(new_content);
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn command_text(&self) -> &str {
        self.command_line.text()
    }

    pub fn clear(&mut self) {
        self.command_line.clear();
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn list_scroll_adjust(&mut self, visible_rows: usize) {
        self.command_list
            .adjust_scroll_for_visible_rows(visible_rows);
    }

    pub fn apply_action(&mut self, action: CommandUiAction, list_rows: usize) {
        match action {
            CommandUiAction::StartPrompt => {
                self.start_prompt();
            }
            CommandUiAction::Clear => {
                self.clear();
            }
            CommandUiAction::InsertChar { ch } => {
                self.command_line.char_insert(ch);
                self.command_list.reset_selection();
                self.focus_on_list = false;
            }
            CommandUiAction::Backspace => {
                self.command_line.backspace();
                self.command_list.reset_selection();
                self.focus_on_list = false;
            }
            CommandUiAction::Delete => {
                self.command_line.delete();
                self.command_list.reset_selection();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveLeft => {
                self.command_line.cursor_move_left();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveRight => {
                self.command_line.cursor_move_right();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveHome => {
                self.command_line.cursor_move_home();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveEnd => {
                self.command_line.cursor_move_end();
                self.focus_on_list = false;
            }
            CommandUiAction::MoveSelectionUp | CommandUiAction::MoveSelectionDown => {
                let matches = self.command_list.filter(self.command_line.text());
                if matches.is_empty() {
                    self.command_list.reset_selection();
                    self.focus_on_list = false;
                    return;
                }

                self.focus_on_list = true;
                match self.command_list.selection() {
                    None => match action {
                        CommandUiAction::MoveSelectionDown => self.command_list.select_index(0),
                        CommandUiAction::MoveSelectionUp => self
                            .command_list
                            .select_index(matches.len().saturating_sub(1)),
                        _ => {}
                    },
                    Some(current_index) => {
                        let max_index = matches.len().saturating_sub(1);
                        match action {
                            CommandUiAction::MoveSelectionUp if current_index > 0 => {
                                self.command_list
                                    .select_index(current_index.saturating_sub(1));
                            }
                            CommandUiAction::MoveSelectionDown if current_index < max_index => {
                                self.command_list
                                    .select_index(current_index.saturating_add(1));
                            }
                            _ => {}
                        }
                    }
                }
                self.command_list.adjust_scroll_for_visible_rows(list_rows);
            }
            CommandUiAction::SelectFromList => {
                let matches = self.command_list.filter(self.command_line.text());
                if self.focus_on_list
                    && !matches.is_empty()
                    && let Some(selected) = self.command_list.selection()
                {
                    let index = selected.min(matches.len().saturating_sub(1));
                    if let Some(entry) = matches.get(index) {
                        self.command_line.set_content(format!(":{}", entry.label()));
                        self.focus_on_list = false;

                        let updated_matches = self.command_list.filter(self.command_line.text());
                        let mut updated_index = None;
                        let mut idx = 0;
                        while idx < updated_matches.len() {
                            if updated_matches[idx].label() == entry.label() {
                                updated_index = Some(idx);
                                break;
                            }
                            idx += 1;
                        }
                        if let Some(updated_index) = updated_index {
                            self.command_list.select_index(updated_index);
                            self.command_list.adjust_scroll_for_visible_rows(list_rows);
                        }
                        return;
                    }
                }
            }
        }
    }

    pub fn snapshot(&self) -> CommandUiSnapshot {
        CommandUiSnapshot {
            command_text: self.command_line.text().to_string(),
            cursor_column: self.command_line.cursor_column(),
            focus_on_list: self.focus_on_list,
            selection: self.command_list.selection(),
            scroll_offset: self.command_list.scroll_position(),
        }
    }

    pub fn frame(&self) -> CommandUiFrame {
        let matches = self.command_list.filter(self.command_line.text());
        let selected_index = if let Some(idx) = self.command_list.selection() {
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
            ));
        }

        CommandUiFrame::new(
            self.command_line.text().to_string(),
            self.command_line.cursor_column(),
            self.focus_on_list,
            list_items,
            selected_index,
            self.command_list.scroll_position(),
        )
    }
}
