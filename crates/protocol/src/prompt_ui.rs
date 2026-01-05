use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptUiFrame {
    line: String,
    cursor_x: u16,
    line_selection: PromptInputSelection,
    line_focus: bool,
    list_items: Vec<PromptListItemFrame>,
    selected_index: Option<usize>,
    scroll_offset: usize,
}

impl PromptUiFrame {
    pub fn new(
        line: String,
        line_focus: bool,
        cursor_x: u16,
        line_selection: PromptInputSelection,
        list_items: Vec<PromptListItemFrame>,
        selected_index: Option<usize>,
        scroll_offset: usize,
    ) -> Self {
        Self {
            line,
            line_focus,
            cursor_x,
            line_selection,
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

    pub fn command_selection(&self) -> PromptInputSelection {
        self.line_selection.clone()
    }

    pub fn line_focus(&self) -> bool {
        self.line_focus
    }

    pub fn command_items(&self) -> &[PromptListItemFrame] {
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
pub enum PromptInputSelection {
    None,
    Range { start: u16, end: u16 },
}

impl PromptInputSelection {
    pub fn range(start: u16, end: u16) -> Self {
        Self::Range { start, end }
    }

    pub fn clear(&mut self) {
        *self = PromptInputSelection::None;
    }

    pub fn indices(&self) -> Vec<usize> {
        match self {
            PromptInputSelection::None => Vec::new(),
            PromptInputSelection::Range { start, end } => {
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
pub enum PromptMode {
    Command,
    Normal,
    Edit,
    Visual,
    PromptCommand,
    PromptKeymap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptListItemFrame {
    name: String,
    description: String,
    mode: PromptMode,
}

impl PromptListItemFrame {
    pub fn new(name: String, description: String, mode: PromptMode) -> Self {
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

    pub fn mode(&self) -> PromptMode {
        self.mode.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PromptUiAction {
    StartPrompt,
    Clear,
    InsertChar { ch: char },
    Backspace,
    Delete,
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    Complete,
    FocusPrompt,
    HistoryPrevious,
    HistoryNext,
    MoveSelectionUp,
    MoveSelectionDown,
    SelectFromList,
}
