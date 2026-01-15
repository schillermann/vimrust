use std::{env, process::Command, thread, time::Duration};

fn main() -> std::io::Result<()> {
    let exe = env::current_exe()?;
    let dir = exe.parent().unwrap();

    let mut core = Command::new(dir.join("core")).spawn()?;
    thread::sleep(Duration::from_millis(200));
    let mut ui = Command::new(dir.join("ui")).spawn()?;

    let _ = ui.wait();
    let _ = core.wait();
    Ok(())
}
