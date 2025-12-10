use std::{
    env, fs,
    io::{self, Write, stdout},
    sync::Mutex,
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

enum EditorMode {
    Normal,
    Edit,
}

impl EditorMode {
    fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Edit => "EDIT",
        }
    }
}

static BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static CURSOR_X: Mutex<u16> = Mutex::new(0);
static CURSOR_Y: Mutex<u16> = Mutex::new(0);
static FILE_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static EDITOR_ROWS_OFFSET: Mutex<u16> = Mutex::new(0);
static EDITOR_COLUMNS_OFFSET: Mutex<u16> = Mutex::new(0);
static VERSION: &str = "0.1.0";
const DEFAULT_TAB_STOP: u16 = 4;

fn char_render_width(character: char, tab_stop: u16, column: u16) -> u16 {
    let tab_size = if tab_stop == 0 { 1 } else { tab_stop };

    match character {
        '\t' => {
            let offset = column % tab_size;
            tab_size.saturating_sub(offset)
        }
        '\x00'..='\x1f' | '\x7f' => 4,
        _ => 1,
    }
}

fn render_segments(line: &str, tab_stop: u16) -> Vec<(u16, u16, char)> {
    let mut segments = Vec::new();
    let mut column: u16 = 0;
    let tab_size = if tab_stop == 0 { 1 } else { tab_stop };

    for ch in line.chars() {
        let start = column;
        let char_width = char_render_width(ch, tab_size, column);
        let end = column.saturating_add(char_width);
        segments.push((start, end, ch));
        column = end;
    }

    segments
}

fn main() -> io::Result<()> {
    let file_path = env::args().nth(1);
    enable_raw_mode()?;
    let result = run(file_path);
    disable_raw_mode()?;
    result
}

fn editor_draw_rows(
    buffer: &mut Vec<u8>,
    number_of_columns: u16,
    number_of_rows: u16,
    columns_offset: u16,
    rows_offset: u16,
    file_lines: &Vec<String>,
) -> io::Result<()> {
    for row_number in 1..=number_of_rows {
        let file_line_number = row_number.saturating_add(rows_offset) as usize;

        queue!(buffer, Clear(ClearType::CurrentLine))?;

        if file_line_number > file_lines.len() {
            queue!(buffer, Print("~"))?;
            if file_lines.is_empty() && row_number == number_of_rows / 3 {
                let mut welcome = format!("VimRust -- version {}", VERSION);
                if welcome.len() > number_of_columns as usize {
                    welcome.truncate(number_of_columns as usize);
                }
                let padding = number_of_columns
                    .saturating_sub(welcome.len() as u16)
                    .saturating_div(2);
                queue!(buffer, MoveTo(padding as u16, row_number), Print(welcome))?;
            }
        } else {
            if let Some(file_line) = file_lines.get(file_line_number.saturating_sub(1)) {
                let displayable_line = displayable_line(file_line, DEFAULT_TAB_STOP);
                let visible_slice: String = displayable_line
                    .chars()
                    .skip(columns_offset as usize)
                    .take(number_of_columns as usize)
                    .collect();
                queue!(buffer, Print(visible_slice))?;
            }
        }

        if row_number < number_of_rows {
            queue!(buffer, Print("\r\n"))?;
        }
    }

    Ok(())
}

fn displayable_line(line: &str, tab_stop: u16) -> String {
    let mut expanded = String::new();
    let mut column: u16 = 0;
    let tab_size = if tab_stop == 0 { 1 } else { tab_stop };

    for ch in line.chars() {
        match ch {
            '\t' => {
                let spaces = char_render_width(ch, tab_size, column);
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

fn terminal_refresh(
    out: &mut io::Stdout,
    terminal_size: (u16, u16),
    cursor_x: u16,
    cursor_y: u16,
    file_lines: &Vec<String>,
    mode: &EditorMode,
) -> io::Result<()> {
    let (number_of_columns, number_of_rows) = terminal_size;
    let usable_rows = number_of_rows.saturating_sub(1);
    let mut columns_offset = EDITOR_COLUMNS_OFFSET.lock().unwrap();
    let mut rows_offset = EDITOR_ROWS_OFFSET.lock().unwrap();
    let mut buffer = BUFFER.lock().unwrap();

    buffer.clear();
    editor_scroll(
        cursor_x,
        cursor_y,
        number_of_columns,
        usable_rows,
        &mut columns_offset,
        &mut rows_offset,
    );

    queue!(&mut *buffer, Hide, MoveTo(0, 0))?;
    editor_draw_rows(
        &mut buffer,
        number_of_columns,
        usable_rows,
        *columns_offset,
        *rows_offset,
        file_lines,
    )?;
    queue!(
        &mut *buffer,
        MoveTo(0, number_of_rows.saturating_sub(1)),
        Clear(ClearType::CurrentLine),
        Print(format!("-- {} --", mode.label()))
    )?;
    queue!(
        &mut *buffer,
        MoveTo(
            cursor_x.saturating_sub(*columns_offset),
            cursor_y.saturating_sub(*rows_offset)
        ),
        Show
    )?;

    out.write_all(&buffer)?;
    out.flush()?;
    Ok(())
}

fn editor_scroll(
    cursor_x: u16,
    cursor_y: u16,
    number_of_columns: u16,
    number_of_rows: u16,
    columns_offset: &mut u16,
    rows_offset: &mut u16,
) {
    if number_of_rows == 0 {
        return;
    }
    if cursor_y < *rows_offset {
        *rows_offset = cursor_y;
    }
    if cursor_y >= (*rows_offset).saturating_add(number_of_rows) {
        *rows_offset = cursor_y.saturating_sub(number_of_rows).saturating_add(1);
    }
    if cursor_x < *columns_offset {
        *columns_offset = cursor_x;
    }
    if cursor_x >= (*columns_offset).saturating_add(number_of_columns) {
        *columns_offset = cursor_x.saturating_sub(number_of_columns).saturating_add(1);
    }
}

fn file_line_length(file_lines: &Vec<String>, cursor_y: u16) -> u16 {
    let line_index = cursor_y as usize;
    if line_index >= file_lines.len() {
        return 0;
    }

    if let Some(line) = file_lines.get(line_index) {
        return visual_line_length(line, DEFAULT_TAB_STOP);
    }

    0
}

fn visual_line_length(line: &str, tab_stop: u16) -> u16 {
    let mut column: u16 = 0;
    let tab_size = if tab_stop == 0 { 1 } else { tab_stop };

    for ch in line.chars() {
        let width = char_render_width(ch, tab_size, column);
        column = column.saturating_add(width);
    }

    column
}

fn next_render_column(line: &str, cursor_x: u16, tab_stop: u16) -> u16 {
    let segments = render_segments(line, tab_stop);
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

fn previous_render_column(line: &str, current_x: u16, tab_stop: u16) -> u16 {
    let segments = render_segments(line, tab_stop);
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

fn snap_cursor_to_render_character(line: &str, cursor_x: u16, tab_stop: u16) -> u16 {
    let segments = render_segments(line, tab_stop);
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

fn render_column_to_char_index(line: &str, cursor_x: u16, tab_stop: u16) -> usize {
    let mut column: u16 = 0;
    let tab_size = if tab_stop == 0 { 1 } else { tab_stop };

    for (idx, ch) in line.char_indices() {
        let width = char_render_width(ch, tab_size, column);
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

fn insert_char_at_cursor(
    file_lines: &mut Vec<String>,
    cursor_x: &mut u16,
    cursor_y: u16,
    ch: char,
) {
    let target_line = cursor_y as usize;
    if target_line >= file_lines.len() {
        file_lines.resize_with(target_line.saturating_add(1), String::new);
    }

    if let Some(line) = file_lines.get_mut(target_line) {
        let insert_at = render_column_to_char_index(line, *cursor_x, DEFAULT_TAB_STOP);
        line.insert(insert_at, ch);
        let advance = char_render_width(ch, DEFAULT_TAB_STOP, *cursor_x);
        *cursor_x = cursor_x.saturating_add(advance);
    }
}

fn editor_move_cursor(
    key_code: KeyCode,
    cursor_x: &mut u16,
    cursor_y: &mut u16,
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let file_lines_len = file_lines.len().min(u16::MAX as usize) as u16;

    match key_code {
        KeyCode::Char('h') => {
            if let Some(line) = file_lines.get(*cursor_y as usize) {
                *cursor_x = previous_render_column(line, *cursor_x, DEFAULT_TAB_STOP);
            } else {
                *cursor_x = cursor_x.saturating_sub(1);
            }
        }
        KeyCode::Char('l') => {
            if let Some(line) = file_lines.get(*cursor_y as usize) {
                *cursor_x = next_render_column(line, *cursor_x, DEFAULT_TAB_STOP);
            } else {
                *cursor_x = cursor_x.saturating_add(1);
            }
        }
        KeyCode::Home => {
            *cursor_x = 0;
        }
        KeyCode::End => {
            *cursor_x = file_line_length(file_lines, *cursor_y);
        }
        KeyCode::Char('k') => {
            *cursor_y = cursor_y.saturating_sub(1);
        }
        KeyCode::Char('j') => {
            *cursor_y = cursor_y.saturating_add(1);
        }
        KeyCode::PageUp => {
            *cursor_y = 0;
        }
        KeyCode::PageDown => {
            *cursor_y = file_lines_len;
        }
        _ => {}
    }

    if *cursor_y > file_lines_len {
        *cursor_y = file_lines_len;
    }

    let line_length = file_line_length(file_lines, *cursor_y);
    if *cursor_x > line_length {
        *cursor_x = line_length;
    }
    if let Some(line) = file_lines.get(*cursor_y as usize) {
        *cursor_x = snap_cursor_to_render_character(line, *cursor_x, DEFAULT_TAB_STOP);
    }

    Ok(())
}

fn editor_open(file_lines: &mut Vec<String>, file_path: String) -> io::Result<()> {
    let contents = fs::read_to_string(file_path)?;
    for line in contents.lines() {
        file_lines.push(line.to_string());
    }
    if file_lines.is_empty() {
        file_lines.push(String::new());
    }
    Ok(())
}

fn run(file_path: Option<String>) -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let result: io::Result<()> = {
        let mut file_lines = FILE_LINES.lock().unwrap();
        if let Some(path) = file_path {
            editor_open(&mut file_lines, path)?;
        }
        if file_lines.is_empty() {
            file_lines.push(String::new());
        }

        let mut terminal_size = size()?;
        let mut cursor_x = CURSOR_X.lock().unwrap();
        let mut cursor_y = CURSOR_Y.lock().unwrap();
        let mut mode = EditorMode::Normal;

        loop {
            terminal_refresh(
                &mut out,
                terminal_size,
                *cursor_x,
                *cursor_y,
                &*file_lines,
                &mode,
            )?;
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => match mode {
                        EditorMode::Normal => match key_event.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('e') => mode = EditorMode::Edit,
                            KeyCode::Esc => mode = EditorMode::Normal,
                            key_code => {
                                editor_move_cursor(
                                    key_code,
                                    &mut cursor_x,
                                    &mut cursor_y,
                                    &*file_lines,
                                )?;
                            }
                        },
                        EditorMode::Edit => match key_event.code {
                            KeyCode::Esc => mode = EditorMode::Normal,
                            KeyCode::Char(ch) => {
                                insert_char_at_cursor(
                                    &mut file_lines,
                                    &mut cursor_x,
                                    *cursor_y,
                                    ch,
                                );
                            }
                            _ => {}
                        },
                    },
                    Event::Resize(columns, rows) => {
                        terminal_size = (columns, rows);
                    }
                    _ => {}
                }
            } else {
                // periodic tasks / redraw can go here
            }
        }
        Ok(())
    };

    let _ = execute!(
        out,
        Clear(ClearType::All),
        MoveTo(0, 0),
        LeaveAlternateScreen
    );
    result
}
