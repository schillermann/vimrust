pub(crate) struct LineView {
    tab_stop: TabStop,
}

impl LineView {
    pub(crate) fn new(tab_size: u16) -> Self {
        Self {
            tab_stop: TabStop::new(tab_size),
        }
    }

    pub(crate) fn displayable_line(&self, line: &str) -> String {
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

    pub(crate) fn char_render_width(&self, character: char, column: u16) -> u16 {
        match character {
            '\t' => self.tab_stop.advance_width(column),
            '\x00'..='\x1f' | '\x7f' => 4,
            _ => 1,
        }
    }

    pub(crate) fn segments_render(&self, line: &str) -> Vec<(u16, u16, char)> {
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

    pub(crate) fn column_next_render(&self, line: &str, cursor_x: u16) -> u16 {
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

                if let Some((_, next_end, next_char)) = next_segment
                    && *next_char == '\t'
                {
                    return next_end.saturating_sub(1);
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

    pub(crate) fn column_previous_render(&self, line: &str, current_x: u16) -> u16 {
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

    pub(crate) fn snap_cursor_to_render_character(&self, line: &str, cursor_x: u16) -> u16 {
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

    pub(crate) fn tab_segment_start(&self, line: &str, cursor_x: u16) -> TabSegmentStart {
        for (start, end, ch) in self.segments_render(line) {
            if cursor_x >= start && cursor_x < end && ch == '\t' {
                return TabSegmentStart::at(start);
            }
        }
        TabSegmentStart::empty()
    }

    pub(crate) fn column_to_char_index_render(&self, line: &str, cursor_x: u16) -> usize {
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

    pub(crate) fn visual_line_length(&self, line: &str) -> u16 {
        let mut column: u16 = 0;

        for ch in line.chars() {
            let width = self.char_render_width(ch, column);
            column = column.saturating_add(width);
        }

        column
    }

    pub(crate) fn line_length(&self, line: &str) -> u16 {
        self.visual_line_length(line)
    }
}

#[derive(Clone, Copy)]
struct TabStop {
    size: u16,
}

pub(crate) struct TabSegmentStart {
    column: u16,
}

impl TabSegmentStart {
    fn empty() -> Self {
        Self {
            column: u16::MAX,
        }
    }

    fn at(column: u16) -> Self {
        Self { column }
    }

    pub(crate) fn apply(&self, cursor: &mut u16) {
        if self.column != u16::MAX {
            *cursor = self.column;
        }
    }
}

impl TabStop {
    fn new(size: u16) -> Self {
        Self { size }
    }

    fn advance_width(&self, column: u16) -> u16 {
        let tab_size = if self.size == 0 { 1 } else { self.size };
        let offset = column % tab_size;
        tab_size.saturating_sub(offset)
    }
}
