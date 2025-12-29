use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandUiFrame {
    line: String,
    cursor_x: u16,
    line_selection: CommandLineSelection,
    focus_on_list: bool,
    list_items: Vec<CommandListItemFrame>,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl CommandUiFrame {
    pub fn new(
        line: String,
        cursor_x: u16,
        line_selection: CommandLineSelection,
        focus_on_list: bool,
        list_items: Vec<CommandListItemFrame>,
        selected_index: Option<usize>,
        scroll_offset: usize,
    ) -> Self {
        Self {
            line,
            cursor_x,
            line_selection,
            focus_on_list,
            list_items,
            selected_index,
            scroll_offset,
        }
    }

    pub fn command_text(&self) -> &str {
        &self.line
    }

    pub fn cursor_column(&self) -> u16 {
        self.cursor_x
    }

    pub fn command_selection(&self) -> CommandLineSelection {
        self.line_selection.clone()
    }

    pub fn list_focus(&self) -> bool {
        self.focus_on_list
    }

    pub fn command_items(&self) -> &[CommandListItemFrame] {
        &self.list_items
    }

    pub fn selected_item(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn scroll_position(&self) -> usize {
        self.scroll_offset
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandLineSelection {
    None,
    Range { start: u16, end: u16 },
}

impl CommandLineSelection {
    pub fn range(start: u16, end: u16) -> Self {
        Self::Range { start, end }
    }

    pub fn clear(&mut self) {
        *self = CommandLineSelection::None;
    }

    pub fn indices(&self) -> Vec<usize> {
        match self {
            CommandLineSelection::None => Vec::new(),
            CommandLineSelection::Range { start, end } => {
                let mut indices = Vec::new();
                let mut idx = *start as usize;
                let end = (*end).max(*start) as usize;
                while idx < end {
                    indices.push(idx);
                    idx = idx.saturating_add(1);
                }
                indices
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandListItemMode {
    Command,
    Normal,
    Edit,
    PromptCommand,
    PromptKeymap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandListItemFrame {
    name: String,
    description: String,
    mode: CommandListItemMode,
}

impl CommandListItemFrame {
    pub fn new(name: String, description: String, mode: CommandListItemMode) -> Self {
        Self {
            name,
            description,
            mode,
        }
    }

    pub fn label(&self) -> &str {
        &self.name
    }

    pub fn detail(&self) -> &str {
        &self.description
    }

    pub fn mode(&self) -> CommandListItemMode {
        self.mode.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
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
