use crate::{
    command_line::CommandLine,
    command_list::CommandList,
};
use vimrust_protocol::{CommandListItemFrame, CommandUiAction, CommandUiFrame};

pub struct CommandUiState {
    command_line: CommandLine,
    command_list: CommandList,
    focus_on_list: bool,
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

    pub fn line(&self) -> &str {
        self.command_line.command_line()
    }

    pub fn clear(&mut self) {
        self.command_line.clear();
        self.command_list.reset_selection();
        self.focus_on_list = false;
    }

    pub fn list_scroll_adjust(&mut self, visible_rows: usize) {
        self.command_list.adjust_scroll_for_visible_rows(visible_rows);
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
        let mut list_items = Vec::with_capacity(matches.len());
        for entry in matches {
            list_items.push(CommandListItemFrame::new(
                entry.name.to_string(),
                entry.description.to_string(),
            ));
        }

        CommandUiFrame::new(
            self.command_line.command_line().to_string(),
            self.command_line.cursor_x(),
            self.focus_on_list,
            list_items,
            selected_index,
            self.command_list.command_scroll_offset(),
        )
    }
}
