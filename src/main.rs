use std::io::{self, stdout, Write};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let result = run();
    disable_raw_mode()?;
    result
}

fn run() -> io::Result<()> {
    let mut out = stdout();
    writeln!(out, "Press keys (q to quit)...")?;
    out.flush()?;

    loop {
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
    }

    Ok(())
}
