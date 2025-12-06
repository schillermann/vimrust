use std::{
    io::{self, Write, stdout},
    sync::Mutex,
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute, queue,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

static BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static CURSOR_POSITION: Mutex<(u16, u16)> = Mutex::new((0, 0));
static FILE_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static EDITOR_ROWS_OFFSET: Mutex<u16> = Mutex::new(0);
static VERSION: &str = "0.1.0";

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn editor_draw_rows(
    buffer: &mut Vec<u8>,
    number_of_columns: u16,
    number_of_rows: u16,
    rows_offset: u16,
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let available_number_of_columns = number_of_columns.saturating_sub(1);
    if file_lines.is_empty() {
        let welcome_row_number = number_of_rows / 3;
        for row_number in 0..number_of_rows {
            queue!(buffer, MoveTo(0, row_number), Clear(ClearType::CurrentLine))?;
            write!(buffer, "~")?;
            if row_number == welcome_row_number {
                let mut welcome = format!("VimRust -- version {}", VERSION);
                if welcome.len() > available_number_of_columns as usize {
                    welcome.truncate(available_number_of_columns as usize);
                }
                let padding = 1
                    + (available_number_of_columns as usize / 2).saturating_sub(welcome.len() / 2);
                let padding = padding.min(available_number_of_columns as usize);
                queue!(buffer, MoveTo(padding as u16, row_number))?;
                write!(buffer, "{}", welcome)?;
            }
        }
    } else {
        for row_number in 0..number_of_rows {
            queue!(buffer, MoveTo(0, row_number), Clear(ClearType::CurrentLine))?;
            let file_index = rows_offset as usize + row_number as usize;
            if let Some(content) = file_lines.get(file_index) {
                let mut line = content.clone();
                if line.len() > available_number_of_columns as usize {
                    line.truncate(available_number_of_columns as usize);
                }
                write!(buffer, "{}", line)?;
            } else {
                write!(buffer, "~")?;
            }
        }
    }
    Ok(())
}

fn editor_scroll(cursor_index_y: u16, number_of_rows: u16, rows_offset: &mut u16) {
    if cursor_index_y < *rows_offset {
        *rows_offset = cursor_index_y;
    }
    if cursor_index_y >= number_of_rows {
        *rows_offset = cursor_index_y
            .saturating_sub(number_of_rows)
            .saturating_add(1);
    }
}

fn terminal_refresh(
    out: &mut io::Stdout,
    terminal_size: (u16, u16),
    cursor_position: (u16, u16),
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let (number_of_columns, number_of_rows) = terminal_size;
    let mut rows_offset = EDITOR_ROWS_OFFSET.lock().unwrap();
    editor_scroll(cursor_position.1, number_of_rows, &mut rows_offset);
    let mut buffer = BUFFER.lock().unwrap();
    buffer.clear();

    queue!(&mut *buffer, Hide, MoveTo(0, 0))?;
    editor_draw_rows(
        &mut buffer,
        number_of_columns,
        number_of_rows,
        *rows_offset,
        file_lines,
    )?;
    queue!(
        &mut *buffer,
        MoveTo(cursor_position.0, cursor_position.1),
        Show
    )?;

    out.write_all(&buffer)?;
    out.flush()
}

fn editor_move_cursor(
    key_code: KeyCode,
    terminal_size: (u16, u16),
    cursor_position: &mut (u16, u16),
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let (number_of_columns, _) = terminal_size;
    let (mut cursor_index_x, mut cursor_index_y) = *cursor_position;

    let cursor_index_x_max = number_of_columns.saturating_sub(1);
    let cursor_index_y_max = (file_lines.len() as u16).saturating_sub(1);

    match key_code {
        KeyCode::Char('h') => {
            if cursor_index_x > 0 {
                cursor_index_x -= 1;
            }
        }
        KeyCode::Char('l') => {
            if cursor_index_x < cursor_index_x_max {
                cursor_index_x += 1;
            }
        }
        KeyCode::Home => {
            cursor_index_x = 0;
        }
        KeyCode::End => {
            cursor_index_x = cursor_index_x_max;
        }
        KeyCode::Char('k') => {
            if cursor_index_y > 0 {
                cursor_index_y -= 1;
            }
        }
        KeyCode::Char('j') => {
            if cursor_index_y < file_lines.len() as u16 {
                cursor_index_y += 1;
            }
        }
        KeyCode::PageUp => {
            cursor_index_y = 0;
        }
        KeyCode::PageDown => {
            cursor_index_y = cursor_index_y_max;
        }
        _ => {}
    }

    *cursor_position = (cursor_index_x, cursor_index_y);
    Ok(())
}

fn editor_open(file_lines: &mut Vec<String>) -> io::Result<()> {
    file_lines.clear();
    file_lines.push("Hello, world!".to_string());
    Ok(())
}

fn run() -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let result: io::Result<()> = {
        let mut file_lines = FILE_LINES.lock().unwrap();
        editor_open(&mut file_lines)?;

        let mut terminal_size = size()?;
        let mut cursor_position = CURSOR_POSITION.lock().unwrap();

        loop {
            terminal_refresh(&mut out, terminal_size, *cursor_position, &file_lines)?;
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if let KeyCode::Char('q') = key_event.code {
                            break;
                        } else {
                            editor_move_cursor(
                                key_event.code,
                                terminal_size,
                                &mut cursor_position,
                                &file_lines,
                            )?;
                        }
                    }
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
