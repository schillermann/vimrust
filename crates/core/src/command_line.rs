pub struct CommandLine {
    content: String,
    cursor_x: u16,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_x: 0,
        }
    }

    pub fn start_prompt(&mut self) {
        self.content.clear();
        self.content.push(':');
        self.cursor_x = 1;
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_x = 0;
    }

    pub fn set_content(&mut self, new_content: String) {
        self.content = new_content;
        self.cursor_x = self.content.len() as u16;
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn cursor_column(&self) -> u16 {
        self.cursor_x
    }

    pub fn backspace(&mut self) {
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
    }

    pub fn cursor_move_right(&mut self) {
        let limit = self.content.len() as u16;
        if self.cursor_x < limit {
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    pub fn cursor_move_home(&mut self) {
        self.cursor_x = 0;
    }

    pub fn cursor_move_end(&mut self) {
        self.cursor_x = self.content.len() as u16;
    }

    pub fn char_insert(&mut self, ch: char) {
        let insert_at = self.cursor_x as usize;
        if insert_at <= self.content.len() {
            self.content.insert(insert_at, ch);
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }
}
