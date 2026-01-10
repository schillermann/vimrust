use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PromptListSelection {
    index: usize,
}

impl PromptListSelection {
    pub fn empty() -> Self {
        Self { index: usize::MAX }
    }

    pub fn at(index: usize) -> Self {
        Self { index }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn clamped(&self, max: usize) -> Self {
        if max == 0 || self.index == usize::MAX {
            return Self::empty();
        }
        Self::at(self.index.min(max.saturating_sub(1)))
    }

    pub fn selected_row(&self, scroll_offset: usize, row_index: usize) -> bool {
        self.index != usize::MAX && self.index == scroll_offset.saturating_add(row_index)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptUiFrame {
    line: String,
    cursor_x: u16,
    line_selection: PromptInputSelection,
    line_focus: bool,
    list_items: Vec<PromptListItemFrame>,
    selected_index: PromptListSelection,
    scroll_offset: usize,
}

impl PromptUiFrame {
    pub fn new(
        line: String,
        line_focus: bool,
        cursor_x: u16,
        line_selection: PromptInputSelection,
        list_items: Vec<PromptListItemFrame>,
        selected_index: PromptListSelection,
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

    pub fn empty() -> Self {
        Self {
            line: String::new(),
            line_focus: false,
            cursor_x: 0,
            line_selection: PromptInputSelection::None,
            list_items: Vec::new(),
            selected_index: PromptListSelection::empty(),
            scroll_offset: 0,
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

    pub fn selection(&self) -> PromptListSelection {
        self.selected_index.clone()
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
