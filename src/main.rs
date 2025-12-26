use std::{env, io};

mod buffer;
mod mode;
mod protocol_guard;
mod rpc_client;
mod rpc_session;
mod status_line;
mod terminal;
mod ui;

use protocol_guard::ProtocolGate;
use rpc_client::ClientFilePath;
use rpc_session::{ModeKeymap, RpcSession};
use terminal::Terminal;
use ui::Ui;
use vimrust_protocol::ProtocolVersion;

fn main() -> io::Result<()> {
    let file_path = ArgFilePath { args: env::args() }.read();

    let mut terminal = Terminal::new()?;
    let result = run_rpc_client(&mut terminal, file_path);
    terminal.cleanup();
    result
}

fn run_rpc_client(terminal: &mut Terminal, file_path: ClientFilePath) -> io::Result<()> {
    let launcher = file_path.launcher();
    let client = launcher.launch()?;
    let ui = Ui::new(terminal);
    let protocol_gate = ProtocolGate::new(ProtocolVersion::current());
    let keymap = ModeKeymap::new();
    let mut session = RpcSession::new(client, ui, protocol_gate, keymap);
    session.open()
}

struct ArgFilePath {
    args: env::Args,
}

impl ArgFilePath {
    fn read(mut self) -> ClientFilePath {
        let _ = self.args.next();
        match self.args.next() {
            Some(path) => ClientFilePath::Provided(path),
            None => ClientFilePath::Missing,
        }
    }
}
