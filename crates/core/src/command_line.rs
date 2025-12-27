use vimrust_protocol::CommandLineSelection;

pub struct CommandLine {
    content: String,
    cursor_x: u16,
    selection: CommandLineSelection,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_x: 0,
            selection: CommandLineSelection::None,
        }
    }

    pub fn start_prompt(&mut self) {
        self.content.clear();
        self.content.push(':');
        self.cursor_x = 1;
        self.selection.clear();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_x = 0;
        self.selection.clear();
    }

    pub fn set_content(&mut self, new_content: String) {
        self.content = new_content;
        self.cursor_x = self.content.len() as u16;
        self.selection.clear();
    }

    pub fn set_content_with_selection(
        &mut self,
        new_content: String,
        selection: CommandLineSelection,
    ) {
        self.content = new_content;
        self.selection = selection;
        self.cursor_x = self.selection_start().min(self.content.len() as u16);
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn cursor_column(&self) -> u16 {
        self.cursor_x
    }

    pub fn selection(&self) -> CommandLineSelection {
        self.selection.clone()
    }

    pub fn backspace(&mut self) {
        if self.selection_active() {
            self.delete_selection();
            return;
        }
        if self.cursor_x == 0 {
            return;
        }
        let delete_at = self.cursor_x.saturating_sub(1) as usize;
        if delete_at < self.content.len() {
            self.content.remove(delete_at);
            self.cursor_x = self.cursor_x.saturating_sub(1);
        }
    }

    pub fn delete(&mut self) {
        if self.selection_active() {
            self.delete_selection();
            return;
        }
        let delete_at = self.cursor_x as usize;
        if delete_at < self.content.len() {
            self.content.remove(delete_at);
            self.cursor_x = self.cursor_x.min(self.content.len() as u16);
        }
    }

    pub fn cursor_move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x = self.cursor_x.saturating_sub(1);
        }
        self.selection.clear();
    }

    pub fn cursor_move_right(&mut self) {
        let limit = self.content.len() as u16;
        if self.cursor_x < limit {
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
        self.selection.clear();
    }

    pub fn cursor_move_home(&mut self) {
        self.cursor_x = 0;
        self.selection.clear();
    }

    pub fn cursor_move_end(&mut self) {
        self.cursor_x = self.content.len() as u16;
        self.selection.clear();
    }

    pub fn char_insert(&mut self, ch: char) {
        if self.selection_active() {
            self.replace_selection(ch);
            return;
        }
        let insert_at = self.cursor_x as usize;
        if insert_at <= self.content.len() {
            self.content.insert(insert_at, ch);
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    fn replace_selection(&mut self, ch: char) {
        let (start, end) = self.selection_range();
        if start >= end {
            self.selection.clear();
            return;
        }
        let mut replacement = String::new();
        replacement.push(ch);
        self.content.replace_range(start..end, &replacement);
        self.cursor_x = start.saturating_add(1) as u16;
        self.selection.clear();
    }

    fn delete_selection(&mut self) {
        let (start, end) = self.selection_range();
        if start >= end {
            self.selection.clear();
            return;
        }
        self.content.replace_range(start..end, "");
        self.cursor_x = start as u16;
        self.selection.clear();
    }

    fn selection_active(&self) -> bool {
        match self.selection {
            CommandLineSelection::None => false,
            CommandLineSelection::Range { start, end } => start < end,
        }
    }

    fn selection_range(&self) -> (usize, usize) {
        match self.selection {
            CommandLineSelection::None => (0, 0),
            CommandLineSelection::Range { start, end } => {
                let max = self.content.len() as u16;
                let start = start.min(max);
                let end = end.min(max).max(start);
                (start as usize, end as usize)
            }
        }
    }

    fn selection_start(&self) -> u16 {
        match self.selection {
            CommandLineSelection::None => self.content.len() as u16,
            CommandLineSelection::Range { start, .. } => start,
        }
    }
}
