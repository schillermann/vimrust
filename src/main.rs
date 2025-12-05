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
        enable_raw_mode,
    },
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn terminal_refresh(out: &mut io::Stdout) -> io::Result<()> {
    execute!(out, Clear(ClearType::All), MoveTo(0, 0))
}

fn run() -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let result: io::Result<()> = {
        writeln!(out, "Press keys (q to quit)...")?;
        out.flush()?;

        loop {
            terminal_refresh(&mut out)?;
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if let KeyCode::Char('q') = key_event.code {
                            break;
                        }
                        writeln!(out, "You pressed: {:?}", key_event.code)?;
                        out.flush()?;
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
