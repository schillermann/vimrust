use std::{
    io::{self, stdout, Write},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn terminalRefresh(out: &mut io::Stdout) -> io::Result<()> {
    execute!(out, Clear(ClearType::All))
}

fn run() -> io::Result<()> {
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    writeln!(out, "Press keys (q to quit)...")?;
    out.flush()?;

    loop {
        terminalRefresh(&mut out)?;
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

    execute!(out, LeaveAlternateScreen)?;
    Ok(())
}
