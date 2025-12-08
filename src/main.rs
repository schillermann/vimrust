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

static BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static CURSOR_X: Mutex<u16> = Mutex::new(0);
static CURSOR_Y: Mutex<u16> = Mutex::new(0);
static FILE_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static EDITOR_ROWS_OFFSET: Mutex<u16> = Mutex::new(0);
static EDITOR_COLUMNS_OFFSET: Mutex<u16> = Mutex::new(0);
static VERSION: &str = "0.1.0";

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
                let mut file_line_excerpt: String =
                    file_line.chars().skip(columns_offset as usize).collect();
                if file_line_excerpt.len() > number_of_columns as usize {
                    file_line_excerpt.truncate(number_of_columns as usize);
                }
                queue!(buffer, Print(file_line_excerpt))?;
            }
        }

        if row_number < number_of_rows {
            queue!(buffer, Print("\r\n"))?;
        }
    }

    Ok(())
}

fn terminal_refresh(
    out: &mut io::Stdout,
    terminal_size: (u16, u16),
    cursor_x: u16,
    cursor_y: u16,
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let (number_of_columns, number_of_rows) = terminal_size;
    let mut columns_offset = EDITOR_COLUMNS_OFFSET.lock().unwrap();
    let mut rows_offset = EDITOR_ROWS_OFFSET.lock().unwrap();
    let mut buffer = BUFFER.lock().unwrap();

    buffer.clear();
    editor_scroll(
        cursor_x,
        cursor_y,
        number_of_columns,
        number_of_rows,
        &mut columns_offset,
        &mut rows_offset,
    );

    queue!(&mut *buffer, Hide, MoveTo(0, 0))?;
    editor_draw_rows(
        &mut buffer,
        number_of_columns,
        number_of_rows,
        *columns_offset,
        *rows_offset,
        file_lines,
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

fn editor_move_cursor(
    key_code: KeyCode,
    terminal_size: (u16, u16),
    cursor_x: &mut u16,
    cursor_y: &mut u16,
    file_lines: &Vec<String>,
) -> io::Result<()> {
    let (number_of_columns, _) = terminal_size;
    let cursor_y_max = (file_lines.len() as u16).saturating_sub(1);

    match key_code {
        KeyCode::Char('h') => {
            if *cursor_x > 0 {
                *cursor_x -= 1;
            }
        }
        KeyCode::Char('l') => {
            *cursor_x += 1;
        }
        KeyCode::Home => {
            *cursor_x = 0;
        }
        KeyCode::End => {
            *cursor_x = number_of_columns.saturating_sub(1);
        }
        KeyCode::Char('k') => {
            if *cursor_y > 0 {
                *cursor_y -= 1;
            }
        }
        KeyCode::Char('j') => {
            if *cursor_y < file_lines.len() as u16 {
                *cursor_y += 1;
            }
        }
        KeyCode::PageUp => {
            *cursor_y = 0;
        }
        KeyCode::PageDown => {
            *cursor_y = cursor_y_max;
        }
        _ => {}
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

        let mut terminal_size = size()?;
        let mut cursor_x = CURSOR_X.lock().unwrap();
        let mut cursor_y = CURSOR_Y.lock().unwrap();

        loop {
            terminal_refresh(&mut out, terminal_size, *cursor_x, *cursor_y, &*file_lines)?;
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if let KeyCode::Char('q') = key_event.code {
                            break;
                        } else {
                            editor_move_cursor(
                                key_event.code,
                                terminal_size,
                                &mut cursor_x,
                                &mut cursor_y,
                                &*file_lines,
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
