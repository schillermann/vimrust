use std::{
    io::{self, Write, stdout},
    time::Duration,
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode, size,
    },
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn editor_draw_rows(out: &mut io::Stdout) -> io::Result<()> {
    let (_, rows) = size()?;
    for row in 0..rows {
        execute!(out, MoveTo(0, row))?;
        write!(out, "~")?;
    }
    Ok(())
}

fn terminal_refresh(out: &mut io::Stdout) -> io::Result<()> {
    execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;
    editor_draw_rows(out)?;
    execute!(out, MoveTo(0, 0))?;
    Ok(())
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
