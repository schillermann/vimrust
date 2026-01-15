use std::{
    env,
    process::{Child, Command},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn main() -> std::io::Result<()> {
    let exe = env::current_exe()?;
    let dir = exe.parent().unwrap();

    let core = Command::new(dir.join("core")).spawn()?;
    thread::sleep(Duration::from_millis(200));
    let ui = Command::new(dir.join("ui")).spawn()?;

    let children = Arc::new(Mutex::new((Some(core), Some(ui))));
    let children_for_handler = Arc::clone(&children);
    ctrlc::set_handler(move || {
        let mut locked = children_for_handler.lock().unwrap();
        terminate_child(locked.0.as_mut());
        terminate_child(locked.1.as_mut());
    })
    .expect("failed to set Ctrl-C handler");

    {
        let mut locked = children.lock().unwrap();
        if let Some(child) = locked.1.as_mut() {
            let _ = child.wait();
        }
        if let Some(child) = locked.0.as_mut() {
            let _ = child.wait();
        }
    }
    Ok(())
}

fn terminate_child(child: Option<&mut Child>) {
    if let Some(child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }
}
