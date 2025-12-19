use serde::{Deserialize, Serialize};

use crate::{command_line::CommandLine, command_list::CommandList};

#[derive(Serialize, Clone)]
pub struct CommandUiFrame {
    pub line: String,
    pub cursor_x: u16,
    pub focus_on_list: bool,
    pub list_items: Vec<CommandListItemFrame>,
    pub selected_index: Option<usize>,
    pub scroll_offset: usize,
}

#[derive(Serialize, Clone)]
pub struct CommandListItemFrame {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CommandUiAction {
    StartPrompt,
    Clear,
    InsertChar { ch: char },
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    MoveSelectionUp,
    MoveSelectionDown,
    SelectFromList,
}

pub struct CommandUiState {
    pub(crate) command_line: CommandLine,
    pub(crate) command_list: CommandList,
    pub(crate) focus_on_list: bool,
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

    pub fn set_line(&mut self, new_content: String) -> bool {
        let changed = self.command_line.command_line() != new_content;
        self.command_line.set_content(new_content);
        self.command_list.reset_selection();
        self.focus_on_list = false;
        changed
    }

    pub fn current_line(&self) -> &str {
        self.command_line.command_line()
    }

    pub fn clear(&mut self) {
        self.command_line.clear();
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn apply_action(&mut self, action: CommandUiAction, list_rows: usize) -> bool {
        match action {
            CommandUiAction::StartPrompt => {
                self.start_prompt();
                true
            }
            CommandUiAction::Clear => {
                self.clear();
                true
            }
            CommandUiAction::InsertChar { ch } => {
                self.command_line.char_insert(ch);
                self.command_list.reset_selection();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::Backspace => {
                self.command_line.backspace();
                self.command_list.reset_selection();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::Delete => {
                self.command_line.delete();
                self.command_list.reset_selection();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::MoveLeft => {
                self.command_line.cursor_move_left();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::MoveRight => {
                self.command_line.cursor_move_right();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::MoveHome => {
                self.command_line.cursor_move_home();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::MoveEnd => {
                self.command_line.cursor_move_end();
                self.focus_on_list = false;
                true
            }
            CommandUiAction::MoveSelectionUp | CommandUiAction::MoveSelectionDown => {
                let matches = self.command_list.filter(self.command_line.command_line());
                if matches.is_empty() {
                    self.command_list.reset_selection();
                    self.focus_on_list = false;
                    return true;
                }

                self.focus_on_list = true;
                match self.command_list.command_selected_index() {
                    None => match action {
                        CommandUiAction::MoveSelectionDown => {
                            self.command_list.set_selected_index(0)
                        }
                        CommandUiAction::MoveSelectionUp => self
                            .command_list
                            .set_selected_index(matches.len().saturating_sub(1)),
                        _ => {}
                    },
                    Some(current_index) => {
                        let max_index = matches.len().saturating_sub(1);
                        match action {
                            CommandUiAction::MoveSelectionUp if current_index > 0 => {
                                self.command_list
                                    .set_selected_index(current_index.saturating_sub(1));
                            }
                            CommandUiAction::MoveSelectionDown if current_index < max_index => {
                                self.command_list
                                    .set_selected_index(current_index.saturating_add(1));
                            }
                            _ => {}
                        }
                    }
                }
                self.command_list.adjust_scroll_for_visible_rows(list_rows);
                true
            }
            CommandUiAction::SelectFromList => {
                let matches = self.command_list.filter(self.command_line.command_line());
                if self.focus_on_list
                    && !matches.is_empty()
                    && let Some(selected) = self.command_list.command_selected_index()
                {
                    let index = selected.min(matches.len().saturating_sub(1));
                    if let Some(entry) = matches.get(index) {
                        self.command_line.set_content(format!(":{}", entry.name));
                        self.focus_on_list = false;

                        let updated_matches =
                            self.command_list.filter(self.command_line.command_line());
                        if let Some(updated_index) = updated_matches
                            .iter()
                            .position(|candidate| candidate.name == entry.name)
                        {
                            self.command_list.set_selected_index(updated_index);
                            self.command_list.adjust_scroll_for_visible_rows(list_rows);
                        }
                        return true;
                    }
                }

                true
            }
        }
    }

    pub fn frame(&self) -> CommandUiFrame {
        let matches = self.command_list.filter(self.command_line.command_line());
        let selected_index = self.command_list.command_selected_index().and_then(|idx| {
            if matches.is_empty() {
                None
            } else {
                Some(idx.min(matches.len().saturating_sub(1)))
            }
        });
        let list_items = matches
            .iter()
            .map(|entry| CommandListItemFrame {
                name: entry.name.to_string(),
                description: entry.description.to_string(),
            })
            .collect();

        CommandUiFrame {
            line: self.command_line.command_line().to_string(),
            cursor_x: self.command_line.cursor_x(),
            focus_on_list: self.focus_on_list,
            list_items,
            selected_index,
            scroll_offset: self.command_list.command_scroll_offset(),
        }
    }
}
