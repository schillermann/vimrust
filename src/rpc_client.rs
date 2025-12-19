use std::{
    io::{self, BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

use crate::rpc::{RpcRequest, RpcResponse};

pub enum ClientEvent {
    Response(RpcResponse),
    Exited,
}

pub struct RpcClient {
    child: Child,
    stdin: ChildStdin,
    pub receiver: Receiver<ClientEvent>,
}

impl RpcClient {
    pub fn spawn(file_path: Option<String>) -> io::Result<Self> {
        let current_exe = std::env::current_exe()?;
        let mut cmd = Command::new(current_exe);
        cmd.arg("--rpc");
        if let Some(path) = file_path {
            cmd.arg(path);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing child stdout"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing child stdin"))?;

        let receiver = Self::spawn_reader(stdout);

        Ok(Self {
            child,
            stdin,
            receiver,
        })
    }

    fn spawn_reader(stdout: ChildStdout) -> Receiver<ClientEvent> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<RpcResponse>(&line) {
                            Ok(resp) => {
                                let _ = tx.send(ClientEvent::Response(resp));
                            }
                            Err(err) => {
                                let _ = tx.send(ClientEvent::Response(RpcResponse::Error {
                                    message: format!("invalid response: {}", err),
                                }));
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = tx.send(ClientEvent::Exited);
        });
        rx
    }

    pub fn send(&mut self, request: &RpcRequest) -> io::Result<()> {
        serde_json::to_writer(&mut self.stdin, request)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}
