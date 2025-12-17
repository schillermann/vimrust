use std::io;

use crossterm::event::KeyCode;

use crate::file::File;

const DEFAULT_TAB_STOP: u16 = 4;
const VERSION: &str = "0.1.0";

pub struct EditorView<'a> {
    pub file: &'a File,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub columns_offset: u16,
    pub rows_offset: u16,
}

pub struct Editor {
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub columns_offset: u16,
    pub rows_offset: u16,
    pub file: File,
    tab_stop: u16,
}

impl Editor {
    pub fn new(file: File) -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            columns_offset: 0,
            rows_offset: 0,
            file,
            tab_stop: DEFAULT_TAB_STOP,
        }
    }

    pub fn view(&self) -> EditorView<'_> {
        EditorView {
            file: &self.file,
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            columns_offset: self.columns_offset,
            rows_offset: self.rows_offset,
        }
    }

    /// Returns a read-only view with scroll offsets adjusted for the viewport.
    pub fn view_with_scroll(&self, number_of_columns: u16, number_of_rows: u16) -> EditorView<'_> {
        let (columns_offset, rows_offset) =
            self.scroll_offsets_compute(number_of_columns, number_of_rows);
        EditorView {
            file: &self.file,
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            columns_offset,
            rows_offset,
        }
    }

    pub fn file_read(&mut self) -> io::Result<()> {
        self.file.read()
    }

    pub fn file_save(&mut self, file_path: &mut Option<String>) -> io::Result<String> {
        let result = self.file.save()?;
        *file_path = self.file.path().cloned();
        Ok(result)
    }

    pub fn scroll(&mut self, number_of_columns: u16, number_of_rows: u16) {
        let (columns_offset, rows_offset) =
            self.scroll_offsets_compute(number_of_columns, number_of_rows);
        self.columns_offset = columns_offset;
        self.rows_offset = rows_offset;
    }

    fn scroll_offsets_compute(&self, number_of_columns: u16, number_of_rows: u16) -> (u16, u16) {
        if number_of_rows == 0 {
            return (self.columns_offset, self.rows_offset);
        }

        let mut rows_offset = self.rows_offset;
        let mut columns_offset = self.columns_offset;

        if self.cursor_y < rows_offset {
            rows_offset = self.cursor_y;
        }
        if self.cursor_y >= rows_offset.saturating_add(number_of_rows) {
            rows_offset = self
                .cursor_y
                .saturating_sub(number_of_rows)
                .saturating_add(1);
        }
        if self.cursor_x < columns_offset {
            columns_offset = self.cursor_x;
        }
        if self.cursor_x >= columns_offset.saturating_add(number_of_columns) {
            columns_offset = self
                .cursor_x
                .saturating_sub(number_of_columns)
                .saturating_add(1);
        }

        (columns_offset, rows_offset)
    }

    pub fn rows_render(
        &self,
        view: &EditorView<'_>,
        number_of_columns: u16,
        number_of_rows: u16,
    ) -> Vec<String> {
        let mut rows = Vec::with_capacity(number_of_rows as usize);
        for row_number in 0..number_of_rows {
            let file_line_number = row_number.saturating_add(view.rows_offset) as usize;

            if file_line_number >= view.file.len() {
                let mut line = String::from("~");
                if view.file.len() == 0 && row_number == number_of_rows / 3 {
                    let mut welcome = format!("VimRust -- version {}", VERSION);
                    if welcome.len() > number_of_columns as usize {
                        welcome.truncate(number_of_columns as usize);
                    }
                    let padding = number_of_columns
                        .saturating_sub(welcome.len() as u16)
                        .saturating_div(2);
                    if padding > 1 {
                        line.push_str(&" ".repeat(padding.saturating_sub(1) as usize));
                    }
                    line.push_str(&welcome);
                }
                if line.len() > number_of_columns as usize {
                    line.truncate(number_of_columns as usize);
                }
                rows.push(line);
            } else if let Some(file_line) = view.file.line(file_line_number) {
                let displayable_line = self.displayable_line(file_line);
                let visible_slice: String = displayable_line
                    .chars()
                    .skip(view.columns_offset as usize)
                    .take(number_of_columns as usize)
                    .collect();
                rows.push(visible_slice);
            }
        }

        rows
    }

    pub fn cursor_move(&mut self, key_code: KeyCode, usable_rows: u16) -> bool {
        let before = (
            self.cursor_x,
            self.cursor_y,
            self.columns_offset,
            self.rows_offset,
        );
        let file_lines_len = self.file.len().min(u16::MAX as usize) as u16;

        match key_code {
            KeyCode::Char('h') => {
                if let Some(line) = self.file.line(self.cursor_y as usize) {
                    self.cursor_x = self.column_previous_render(line, self.cursor_x);
                } else {
                    self.cursor_x = self.cursor_x.saturating_sub(1);
                }
            }
            KeyCode::Char('l') => {
                if let Some(line) = self.file.line(self.cursor_y as usize) {
                    self.cursor_x = self.column_next_render(line, self.cursor_x);
                } else {
                    self.cursor_x = self.cursor_x.saturating_add(1);
                }
            }
            KeyCode::Home => {
                self.cursor_x = 0;
            }
            KeyCode::End => {
                self.cursor_x = self.file_line_length(self.cursor_y);
            }
            KeyCode::Char('k') => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Char('j') => {
                self.cursor_y = self.cursor_y.saturating_add(1);
            }
            KeyCode::PageUp => {
                if usable_rows == 0 {
                    self.cursor_y = 0;
                    self.rows_offset = 0;
                } else {
                    let new_cursor_y = self.cursor_y.saturating_sub(usable_rows);
                    let lower_third = usable_rows.saturating_mul(2).saturating_div(3);
                    let new_offset = new_cursor_y.saturating_sub(lower_third);
                    self.cursor_y = new_cursor_y;
                    self.rows_offset = new_offset;
                }
            }
            KeyCode::PageDown => {
                if usable_rows == 0 {
                    self.cursor_y = file_lines_len;
                } else {
                    let new_cursor_y = self
                        .cursor_y
                        .saturating_add(usable_rows)
                        .min(file_lines_len);
                    let upper_third = usable_rows.saturating_div(3);
                    let new_offset = new_cursor_y.saturating_sub(upper_third);
                    self.cursor_y = new_cursor_y;
                    self.rows_offset = new_offset;
                }
            }
            _ => {}
        }

        if self.cursor_y > file_lines_len {
            self.cursor_y = file_lines_len;
        }

        let line_length = self.file_line_length(self.cursor_y);
        if self.cursor_x > line_length {
            self.cursor_x = line_length;
        }
        if let Some(line) = self.file.line(self.cursor_y as usize) {
            self.cursor_x = self.snap_cursor_to_render_character(line, self.cursor_x);
        }

        (
            self.cursor_x,
            self.cursor_y,
            self.columns_offset,
            self.rows_offset,
        ) != before
    }

    pub fn char_insert(&mut self, ch: char) -> bool {
        let target_line = self.cursor_y as usize;
        if target_line >= self.file.len() {
            self.file
                .file_lines
                .resize_with(target_line.saturating_add(1), String::new);
        }

        let insert_at = match self.file.line(target_line) {
            Some(line) => self.column_to_char_index_render(line, self.cursor_x),
            None => 0,
        };
        let advance = self.char_render_width(ch, self.cursor_x);

        if let Some(line) = self.file.file_lines.get_mut(target_line) {
            let previous_len = line.len();
            let previous_cursor_x = self.cursor_x;
            line.insert(insert_at, ch);
            self.cursor_x = self.cursor_x.saturating_add(advance);
            return line.len() != previous_len || self.cursor_x != previous_cursor_x;
        }

        false
    }

    pub fn backspace_delete(&mut self) -> bool {
        if self.cursor_x == 0 && self.cursor_y == 0 {
            return false;
        }

        if self.cursor_x == 0 {
            let current_index = self.cursor_y as usize;
            if current_index == 0 || current_index >= self.file.len() {
                return false;
            }
            if let Some(current_line) = self.file.line(current_index).cloned() {
                let new_cursor_x = match self.file.line(current_index.saturating_sub(1)) {
                    Some(prev) => self.visual_line_length(prev),
                    None => 0,
                };
                if let Some(previous_line) = self
                    .file
                    .file_lines
                    .get_mut(current_index.saturating_sub(1))
                {
                    previous_line.push_str(&current_line);
                    self.file.file_lines.remove(current_index);
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                    self.cursor_x = new_cursor_x;
                    return true;
                }
            }
            return false;
        }

        let (new_cursor_x, delete_idx) = match self.file.line(self.cursor_y as usize) {
            Some(line) => (
                self.column_previous_render(line, self.cursor_x),
                self.column_to_char_index_render(
                    line,
                    self.column_previous_render(line, self.cursor_x),
                ),
            ),
            None => (0, 0),
        };

        if let Some(line) = self.file.file_lines.get_mut(self.cursor_y as usize) {
            if delete_idx < line.len() {
                line.remove(delete_idx);
                self.cursor_x = new_cursor_x;
                return true;
            }
        }

        false
    }

    pub fn under_cursor_delete(&mut self) -> bool {
        let delete_idx = match self.file.line(self.cursor_y as usize) {
            Some(line) => self.column_to_char_index_render(line, self.cursor_x),
            None => return false,
        };

        if let Some(line) = self.file.file_lines.get_mut(self.cursor_y as usize) {
            if delete_idx < line.len() {
                line.remove(delete_idx);
                return true;
            }
        } else {
            return false;
        }

        let current_index = self.cursor_y as usize;
        if current_index + 1 < self.file.len() {
            if let Some(next_line) = self.file.line(current_index + 1).cloned() {
                if let Some(current_line) = self.file.file_lines.get_mut(current_index) {
                    current_line.push_str(&next_line);
                }
                self.file.file_lines.remove(current_index + 1);
                return true;
            }
        }

        false
    }

    pub fn snap_cursor_to_tab_start(&mut self) {
        if let Some(line) = self.file.line(self.cursor_y as usize) {
            if let Some(start) = self.tab_segment_start(line, self.cursor_x) {
                self.cursor_x = start;
            }
        }
    }

    fn file_line_length(&self, cursor_y: u16) -> u16 {
        let line_index = cursor_y as usize;
        if line_index >= self.file.len() {
            return 0;
        }

        if let Some(line) = self.file.line(line_index) {
            return self.line_length(line);
        }

        0
    }

    fn displayable_line(&self, line: &str) -> String {
        let mut expanded = String::new();
        let mut column: u16 = 0;

        for ch in line.chars() {
            match ch {
                '\t' => {
                    let spaces = self.char_render_width(ch, column);
                    let mut count = 0;
                    while count < spaces {
                        expanded.push(' ');
                        count += 1;
                    }
                    column = column.saturating_add(spaces);
                }
                '\x00'..='\x1f' => {
                    let hex = format!("<{:02X}>", ch as u8);
                    expanded.push_str(&hex);
                    column = column.saturating_add(4);
                }
                '\x7f' => {
                    expanded.push_str("<7F>");
                    column = column.saturating_add(4);
                }
                _ => {
                    expanded.push(ch);
                    column = column.saturating_add(1);
                }
            }
        }

        expanded
    }

    fn char_render_width(&self, character: char, column: u16) -> u16 {
        let tab_size = if self.tab_stop == 0 { 1 } else { self.tab_stop };

        match character {
            '\t' => {
                let offset = column % tab_size;
                tab_size.saturating_sub(offset)
            }
            '\x00'..='\x1f' | '\x7f' => 4,
            _ => 1,
        }
    }

    fn segments_render(&self, line: &str) -> Vec<(u16, u16, char)> {
        let mut segments = Vec::new();
        let mut column: u16 = 0;

        for ch in line.chars() {
            let start = column;
            let char_width = self.char_render_width(ch, column);
            let end = column.saturating_add(char_width);
            segments.push((start, end, ch));
            column = end;
        }

        segments
    }

    fn column_next_render(&self, line: &str, cursor_x: u16) -> u16 {
        let segments = self.segments_render(line);
        if segments.is_empty() {
            return 0;
        }

        for (idx, (start, end, ch)) in segments.iter().enumerate() {
            let next_segment = segments.get(idx.saturating_add(1));

            if cursor_x < *start {
                if *ch == '\t' {
                    return end.saturating_sub(1);
                }
                return *start;
            }

            if cursor_x < *end {
                if *ch == '\t' {
                    let target = end.saturating_sub(1);
                    if cursor_x < target {
                        return target;
                    }
                    if let Some((next_start, next_end, next_ch)) = next_segment {
                        if *next_ch == '\t' {
                            return next_end.saturating_sub(1);
                        }
                        return *next_start;
                    }
                    return *end;
                }

                if let Some((_, next_end, next_char)) = next_segment {
                    if *next_char == '\t' {
                        return next_end.saturating_sub(1);
                    }
                }

                return *end;
            }
        }

        if let Some((_, end, _)) = segments.last() {
            *end
        } else {
            0
        }
    }

    fn column_previous_render(&self, line: &str, current_x: u16) -> u16 {
        let segments = self.segments_render(line);
        if segments.is_empty() {
            return 0;
        }

        let mut best: u16 = 0;
        for (start, end, ch) in segments {
            let stop = if ch == '\t' {
                end.saturating_sub(1)
            } else {
                start
            };

            if stop < current_x && stop >= best {
                best = stop;
            }
        }

        best
    }

    fn snap_cursor_to_render_character(&self, line: &str, cursor_x: u16) -> u16 {
        let segments = self.segments_render(line);
        if segments.is_empty() {
            return 0;
        }

        let line_length = match segments.last() {
            Some((_, end, _)) => *end,
            None => 0,
        };
        let clamped_x = cursor_x.min(line_length);
        let last_index = segments.len() - 1;

        for (idx, (start, end, ch)) in segments.iter().enumerate() {
            let in_segment = clamped_x >= *start && clamped_x < *end;
            let at_line_end = clamped_x == line_length && idx == last_index;

            if in_segment {
                return match ch {
                    '\t' => end.saturating_sub(1),
                    '\x00'..='\x1f' | '\x7f' => *start,
                    _ => *start,
                };
            }

            if at_line_end {
                return match ch {
                    '\t' => end.saturating_sub(1),
                    '\x00'..='\x1f' | '\x7f' => *start,
                    _ => clamped_x,
                };
            }
        }

        clamped_x
    }

    fn tab_segment_start(&self, line: &str, cursor_x: u16) -> Option<u16> {
        for (start, end, ch) in self.segments_render(line) {
            if cursor_x >= start && cursor_x < end && ch == '\t' {
                return Some(start);
            }
        }
        None
    }

    fn column_to_char_index_render(&self, line: &str, cursor_x: u16) -> usize {
        let mut column: u16 = 0;

        for (idx, ch) in line.char_indices() {
            let width = self.char_render_width(ch, column);
            if cursor_x <= column {
                return idx;
            }
            if cursor_x < column.saturating_add(width) {
                return idx;
            }
            column = column.saturating_add(width);
        }

        line.len()
    }

    fn visual_line_length(&self, line: &str) -> u16 {
        let mut column: u16 = 0;

        for ch in line.chars() {
            let width = self.char_render_width(ch, column);
            column = column.saturating_add(width);
        }

        column
    }

    fn line_length(&self, line: &str) -> u16 {
        // Wrapper kept separate in case line length rules diverge from visual length later.
        self.visual_line_length(line)
    }
}
