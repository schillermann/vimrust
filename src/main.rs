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
) -> io::Result<()> {
    let welcome_row_number = number_of_rows / 3;
    for row_number in 0..number_of_rows {
        queue!(buffer, MoveTo(0, row_number), Clear(ClearType::CurrentLine))?;
        write!(buffer, "~")?;
        if row_number == welcome_row_number {
            let mut welcome = format!("VimRust -- version {}", VERSION);
            let available_number_of_columns = number_of_columns - 1; // leave space for the leading tilde
            if welcome.len() > available_number_of_columns as usize {
                welcome.truncate(available_number_of_columns as usize);
            }
            let padding =
                1 + (available_number_of_columns as usize / 2).saturating_sub(welcome.len() / 2);
            let padding = padding.min(available_number_of_columns as usize);
            queue!(buffer, MoveTo(padding as u16, row_number))?;
            write!(buffer, "{}", welcome)?;
        }
    }
    Ok(())
}

fn terminal_refresh(out: &mut io::Stdout) -> io::Result<()> {
    let (number_of_columns, number_of_rows) = size()?;
    let mut buffer = BUFFER.lock().unwrap();
    buffer.clear();

    queue!(&mut *buffer, Hide, MoveTo(0, 0))?;
    editor_draw_rows(&mut buffer, number_of_columns, number_of_rows)?;
    queue!(&mut *buffer, MoveTo(0, 0), Show)?;

    out.write_all(&buffer)?;
    out.flush()
}

fn run() -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let result: io::Result<()> = {
        loop {
            terminal_refresh(&mut out)?;
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if let KeyCode::Char('q') = key_event.code {
                            break;
                        }
                    }
                    Event::Resize(_, _) => {}
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
