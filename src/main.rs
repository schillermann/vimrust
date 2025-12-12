use std::{
    env, fs,
    io::{self, Write, stdout},
    sync::Mutex,
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{self, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

enum EditorMode {
    Normal,
    Edit,
    Command,
}

impl EditorMode {
    fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Edit => "EDIT",
            EditorMode::Command => "COMMAND",
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
const DEFAULT_STATUS: &str = "| e: edit | Esc: normal | s: save | q: quit";
const DEFAULT_COMMAND_PLACEHOLDER: &str = "Press : for commands";

struct CommandEntry {
    name: &'static str,
    description: &'static str,
}

static COMMANDS: &[CommandEntry] = &[
    CommandEntry {
        name: "s",
        description: "Save the current buffer",
    },
    CommandEntry {
        name: "save",
        description: "Save the current buffer",
    },
    CommandEntry {
        name: "q",
        description: "Quit the editor",
    },
    CommandEntry {
        name: "quit",
        description: "Quit the editor",
    },
    CommandEntry {
        name: "sq",
        description: "Save and quit",
    },
    CommandEntry {
        name: "o filename",
        description: "Open a file",
    },
    CommandEntry {
        name: "open filename",
        description: "Open a file",
    },
];

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

fn draw_command_line(
    buffer: &mut Vec<u8>,
    number_of_columns: u16,
    command_line: &str,
) -> io::Result<()> {
    queue!(buffer, MoveTo(0, 0), Clear(ClearType::CurrentLine))?;
    let is_placeholder = command_line.is_empty();
    let display_content = if is_placeholder {
        DEFAULT_COMMAND_PLACEHOLDER
    } else {
        command_line
    };

    let mut visible: String = display_content
        .chars()
        .take(number_of_columns as usize)
        .collect();
    if visible.len() < number_of_columns as usize {
        visible.push_str(&" ".repeat(number_of_columns as usize - visible.len()));
    }
    queue!(
        buffer,
        SetBackgroundColor(Color::Rgb {
            r: 27,
            g: 27,
            b: 27
        }),
        SetForegroundColor(if is_placeholder {
            Color::DarkGrey
        } else {
            Color::Grey
        }),
        Print(visible),
        ResetColor
    )?;
    Ok(())
}

fn editor_draw_rows(
    buffer: &mut Vec<u8>,
    number_of_columns: u16,
    number_of_rows: u16,
    columns_offset: u16,
    rows_offset: u16,
    file_lines: &Vec<String>,
    start_row: u16,
) -> io::Result<()> {
    for row_number in 0..number_of_rows {
        let screen_row = start_row.saturating_add(row_number);
        let file_line_number = row_number.saturating_add(rows_offset) as usize;

        queue!(buffer, MoveTo(0, screen_row), Clear(ClearType::CurrentLine))?;

        if file_line_number >= file_lines.len() {
            queue!(buffer, Print("~"))?;
            if file_lines.is_empty() && row_number == number_of_rows / 3 {
                let mut welcome = format!("VimRust -- version {}", VERSION);
                if welcome.len() > number_of_columns as usize {
                    welcome.truncate(number_of_columns as usize);
                }
                let padding = number_of_columns
                    .saturating_sub(welcome.len() as u16)
                    .saturating_div(2);
                queue!(buffer, MoveTo(padding as u16, screen_row), Print(welcome))?;
            }
        } else if let Some(file_line) = file_lines.get(file_line_number) {
            let displayable_line = displayable_line(file_line, DEFAULT_TAB_STOP);
            let visible_slice: String = displayable_line
                .chars()
                .skip(columns_offset as usize)
                .take(number_of_columns as usize)
                .collect();
            queue!(buffer, Print(visible_slice))?;
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

fn fit_status(message: &str, max_width: u16) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut truncated = message.to_string();
    if truncated.len() > max_width as usize {
        truncated.truncate(max_width as usize);
    }
    truncated
}

fn terminal_refresh(
    out: &mut io::Stdout,
    terminal_size: (u16, u16),
    cursor_x: u16,
    cursor_y: u16,
    file_lines: &Vec<String>,
    mode: &EditorMode,
    status_message: &str,
    command_line: &str,
    command_cursor_x: u16,
    command_selected_index: usize,
    command_scroll_offset: usize,
    command_focus_on_list: bool,
) -> io::Result<()> {
    let (number_of_columns, number_of_rows) = terminal_size;
    if number_of_rows == 0 {
        return Ok(());
    }

    let usable_rows = number_of_rows.saturating_sub(2);
    let mut columns_offset = EDITOR_COLUMNS_OFFSET.lock().unwrap();
    let mut rows_offset = EDITOR_ROWS_OFFSET.lock().unwrap();
    let mut buffer = BUFFER.lock().unwrap();

    buffer.clear();
    queue!(&mut *buffer, Hide)?;

    draw_command_line(&mut buffer, number_of_columns, command_line)?;

    if usable_rows > 0 {
        editor_scroll(
            cursor_x,
            cursor_y,
            number_of_columns,
            usable_rows,
            &mut columns_offset,
            &mut rows_offset,
        );

        editor_draw_rows(
            &mut buffer,
            number_of_columns,
            usable_rows,
            *columns_offset,
            *rows_offset,
            file_lines,
            1,
        )?;
    }
    // If the terminal is too small, the command line must not be overwritten by the status line.
    if number_of_rows > 1 {
        let mut status = format!(
            "-- {} -- {}",
            mode.label(),
            fit_status(
                status_message,
                number_of_columns
                    .saturating_sub(mode.label().len() as u16)
                    .saturating_sub(6)
            )
        );
        if status.len() < number_of_columns as usize {
            status.push_str(&" ".repeat(number_of_columns as usize - status.len()));
        } else {
            status.truncate(number_of_columns as usize);
        }
        queue!(
            &mut *buffer,
            MoveTo(0, number_of_rows.saturating_sub(1)),
            Clear(ClearType::CurrentLine),
            SetBackgroundColor(Color::Grey),
            SetForegroundColor(Color::Black),
            Print(status),
            ResetColor
        )?;
    }
    let (cursor_col, cursor_row) = match mode {
        EditorMode::Command if command_focus_on_list => {
            let relative_row = command_selected_index.saturating_sub(command_scroll_offset) as u16;
            let list_row = 1u16
                .saturating_add(1)
                .saturating_add(2)
                .saturating_add(relative_row)
                .min(number_of_rows.saturating_sub(1));
            (0, list_row)
        }
        EditorMode::Command => (command_cursor_x.min(number_of_columns.saturating_sub(1)), 0),
        _ => (
            cursor_x
                .saturating_sub(*columns_offset)
                .min(number_of_columns.saturating_sub(1)),
            cursor_y
                .saturating_sub(*rows_offset)
                .saturating_add(1)
                .min(number_of_rows.saturating_sub(1)),
        ),
    };
    queue!(&mut *buffer, MoveTo(cursor_col, cursor_row), Show)?;

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

fn tab_segment_start(line: &str, cursor_x: u16, tab_stop: u16) -> Option<u16> {
    for (start, end, ch) in render_segments(line, tab_stop) {
        if cursor_x >= start && cursor_x < end && ch == '\t' {
            return Some(start);
        }
    }
    None
}

fn snap_cursor_to_tab_start(
    file_lines: &Vec<String>,
    cursor_x: &mut u16,
    cursor_y: u16,
    tab_stop: u16,
) {
    if let Some(line) = file_lines.get(cursor_y as usize) {
        if let Some(start) = tab_segment_start(line, *cursor_x, tab_stop) {
            *cursor_x = start;
        }
    }
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

fn delete_backspace(file_lines: &mut Vec<String>, cursor_x: &mut u16, cursor_y: &mut u16) {
    // If at the very start of the buffer, nothing to do.
    if *cursor_x == 0 && *cursor_y == 0 {
        return;
    }

    // Merge with previous line when at column 0.
    if *cursor_x == 0 {
        let current_index = *cursor_y as usize;
        if current_index == 0 || current_index >= file_lines.len() {
            return;
        }
        if let Some(current_line) = file_lines.get(current_index).cloned() {
            if let Some(previous_line) = file_lines.get_mut(current_index.saturating_sub(1)) {
                let new_cursor_x = visual_line_length(previous_line, DEFAULT_TAB_STOP);
                previous_line.push_str(&current_line);
                file_lines.remove(current_index);
                *cursor_y = cursor_y.saturating_sub(1);
                *cursor_x = new_cursor_x;
            }
        }
        return;
    }

    // Delete character to the left within the line.
    if let Some(line) = file_lines.get_mut(*cursor_y as usize) {
        let new_cursor_x = previous_render_column(line, *cursor_x, DEFAULT_TAB_STOP);
        let delete_idx = render_column_to_char_index(line, new_cursor_x, DEFAULT_TAB_STOP);
        if delete_idx < line.len() {
            line.remove(delete_idx);
            *cursor_x = new_cursor_x;
        }
    }
}

fn delete_under_cursor(file_lines: &mut Vec<String>, cursor_x: &mut u16, cursor_y: &mut u16) {
    if let Some(line) = file_lines.get_mut(*cursor_y as usize) {
        let delete_idx = render_column_to_char_index(line, *cursor_x, DEFAULT_TAB_STOP);
        if delete_idx < line.len() {
            line.remove(delete_idx);
            return;
        }
    } else {
        return;
    }

    // At end of line: merge with next line if it exists.
    let current_index = *cursor_y as usize;
    if current_index + 1 < file_lines.len() {
        if let Some(next_line) = file_lines.get(current_index + 1).cloned() {
            if let Some(current_line) = file_lines.get_mut(current_index) {
                current_line.push_str(&next_line);
            }
            file_lines.remove(current_index + 1);
        }
    }
}

fn editor_save(file_lines: &Vec<String>, file_path: &mut Option<String>) -> io::Result<String> {
    let path = file_path
        .get_or_insert_with(|| String::from("untitled.txt"))
        .clone();
    let contents = file_lines.join("\n");
    fs::write(&path, contents)?;
    Ok(format!("Wrote {}", path))
}

fn set_cursor_style(out: &mut io::Stdout, mode: &EditorMode) -> io::Result<()> {
    let style = match mode {
        EditorMode::Normal => SetCursorStyle::DefaultUserShape,
        EditorMode::Edit => SetCursorStyle::SteadyBar,
        EditorMode::Command => SetCursorStyle::SteadyBar,
    };
    execute!(out, style)
}

fn editor_move_cursor(
    key_code: KeyCode,
    cursor_x: &mut u16,
    cursor_y: &mut u16,
    file_lines: &Vec<String>,
    usable_rows: u16,
    rows_offset: &mut u16,
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
            if usable_rows == 0 {
                *cursor_y = 0;
                *rows_offset = 0;
            } else {
                let new_cursor_y = cursor_y.saturating_sub(usable_rows);
                let lower_third = usable_rows.saturating_mul(2).saturating_div(3);
                let new_offset = new_cursor_y.saturating_sub(lower_third);
                *cursor_y = new_cursor_y;
                *rows_offset = new_offset;
            }
        }
        KeyCode::PageDown => {
            if usable_rows == 0 {
                *cursor_y = file_lines_len;
            } else {
                let new_cursor_y = cursor_y.saturating_add(usable_rows).min(file_lines_len);
                let upper_third = usable_rows.saturating_div(3);
                let new_offset = new_cursor_y.saturating_sub(upper_third);
                *cursor_y = new_cursor_y;
                *rows_offset = new_offset;
            }
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

fn update_status(current: &mut String, needs_refresh: &mut bool, message: String) {
    if *current != message {
        *current = message;
        *needs_refresh = true;
    }
}

fn run(mut file_path: Option<String>) -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let result: io::Result<()> = {
        let mut file_lines = FILE_LINES.lock().unwrap();
        if let Some(path) = file_path.clone() {
            editor_open(&mut file_lines, path)?;
        }
        if file_lines.is_empty() {
            file_lines.push(String::new());
        }

        let mut terminal_size = size()?;
        let mut cursor_x = CURSOR_X.lock().unwrap();
        let mut cursor_y = CURSOR_Y.lock().unwrap();
        let mut mode = EditorMode::Normal;
        let mut status_message = String::from(DEFAULT_STATUS);
        let mut needs_refresh = true;
        let mut command_line = String::new();
        let mut command_cursor_x: u16 = 0;
        let mut command_selected_index: usize = 0;
        let mut command_scroll_offset: usize = 0;
        let mut command_focus_on_list: bool = false;
        set_cursor_style(&mut out, &mode)?;

        loop {
            if needs_refresh {
                terminal_refresh(
                    &mut out,
                    terminal_size,
                    *cursor_x,
                    *cursor_y,
                    &*file_lines,
                    &mode,
                    &status_message,
                    &command_line,
                    command_cursor_x,
                    command_selected_index,
                    command_scroll_offset,
                    command_focus_on_list,
                )?;
                needs_refresh = false;
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        needs_refresh = true;
                        match mode {
                            EditorMode::Normal => match key_event.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('e') => {
                                    mode = EditorMode::Edit;
                                    set_cursor_style(&mut out, &mode)?;
                                    snap_cursor_to_tab_start(
                                        &*file_lines,
                                        &mut cursor_x,
                                        *cursor_y,
                                        DEFAULT_TAB_STOP,
                                    );
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Esc => {
                                    mode = EditorMode::Normal;
                                    set_cursor_style(&mut out, &mode)?;
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Char('s') => {
                                    match editor_save(&*file_lines, &mut file_path) {
                                        Ok(msg) => update_status(
                                            &mut status_message,
                                            &mut needs_refresh,
                                            msg,
                                        ),
                                        Err(err) => update_status(
                                            &mut status_message,
                                            &mut needs_refresh,
                                            format!("Error saving: {}", err),
                                        ),
                                    }
                                }
                                KeyCode::Char(':') => {
                                    mode = EditorMode::Command;
                                    command_line.clear();
                                    command_line.push(':');
                                    command_cursor_x = 1;
                                    command_selected_index = 0;
                                    command_scroll_offset = 0;
                                    command_focus_on_list = false;
                                    set_cursor_style(&mut out, &mode)?;
                                }
                                key_code => {
                                    let usable_rows = terminal_size.1.saturating_sub(2);
                                    let mut rows_offset = EDITOR_ROWS_OFFSET.lock().unwrap();
                                    editor_move_cursor(
                                        key_code,
                                        &mut cursor_x,
                                        &mut cursor_y,
                                        &*file_lines,
                                        usable_rows,
                                        &mut *rows_offset,
                                    )?;
                                }
                            },
                            EditorMode::Edit => match key_event.code {
                                KeyCode::Esc => {
                                    mode = EditorMode::Normal;
                                    set_cursor_style(&mut out, &mode)?;
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Delete => {
                                    delete_under_cursor(
                                        &mut file_lines,
                                        &mut cursor_x,
                                        &mut cursor_y,
                                    );
                                }
                                KeyCode::Backspace => {
                                    delete_backspace(&mut file_lines, &mut cursor_x, &mut cursor_y);
                                }
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
                            EditorMode::Command => match key_event.code {
                                KeyCode::Esc => {
                                    mode = EditorMode::Normal;
                                    set_cursor_style(&mut out, &mode)?;
                                    command_line.clear();
                                    command_cursor_x = 0;
                                    command_selected_index = 0;
                                    command_scroll_offset = 0;
                                    command_focus_on_list = false;
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Backspace => {
                                    if command_cursor_x > 0 {
                                        let delete_at = command_cursor_x.saturating_sub(1) as usize;
                                        if delete_at < command_line.len() {
                                            command_line.remove(delete_at);
                                            command_cursor_x = command_cursor_x.saturating_sub(1);
                                            command_selected_index = 0;
                                            command_scroll_offset = 0;
                                            command_focus_on_list = false;
                                        }
                                    }
                                }
                                KeyCode::Delete => {
                                    let delete_at = command_cursor_x as usize;
                                    if delete_at < command_line.len() {
                                        command_line.remove(delete_at);
                                        command_selected_index = 0;
                                        command_scroll_offset = 0;
                                        command_focus_on_list = false;
                                    }
                                    command_cursor_x =
                                        command_cursor_x.min(command_line.len() as u16);
                                }
                                KeyCode::Left => {
                                    if command_cursor_x > 0 {
                                        command_cursor_x = command_cursor_x.saturating_sub(1);
                                    }
                                    command_focus_on_list = false;
                                }
                                KeyCode::Right => {
                                    let limit = command_line.len() as u16;
                                    if command_cursor_x < limit {
                                        command_cursor_x = command_cursor_x.saturating_add(1);
                                    }
                                    command_focus_on_list = false;
                                }
                                KeyCode::Home => {
                                    command_cursor_x = 0;
                                    command_focus_on_list = false;
                                }
                                KeyCode::End => {
                                    command_cursor_x = command_line.len() as u16;
                                    command_focus_on_list = false;
                                }
                                KeyCode::Char(ch) => {
                                    let insert_at = command_cursor_x as usize;
                                    if insert_at <= command_line.len() {
                                        command_line.insert(insert_at, ch);
                                        command_cursor_x = command_cursor_x.saturating_add(1);
                                        command_selected_index = 0;
                                        command_scroll_offset = 0;
                                        command_focus_on_list = false;
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                    Event::Resize(columns, rows) => {
                        terminal_size = (columns, rows);
                        needs_refresh = true;
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
