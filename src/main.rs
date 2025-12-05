use std::{io::{self, stdout, Write}, sync::Mutex, time::Duration};

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

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn editor_draw_rows(buffer: &mut Vec<u8>, rows: u16) -> io::Result<()> {
    for row in 0..rows {
        queue!(buffer, MoveTo(0, row))?;
        write!(buffer, "~")?;
    }
    Ok(())
}

fn terminal_refresh(out: &mut io::Stdout) -> io::Result<()> {
    let (_, rows) = size()?;
    let mut buffer = BUFFER.lock().unwrap();
    buffer.clear();

    queue!(&mut *buffer, Hide, Clear(ClearType::All), MoveTo(0, 0))?;
    editor_draw_rows(&mut buffer, rows)?;
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
