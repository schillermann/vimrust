use std::{
    io::{self, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use vimrust_protocol::{RpcRequest, RpcResponse};

struct MissingPipe {
    name: &'static str,
}

impl MissingPipe {
    fn to_error(&self) -> io::Error {
        io::Error::new(io::ErrorKind::Other, format!("missing child {}", self.name))
    }
}

pub enum ClientEvent {
    Response(RpcResponse),
    Exited,
}

pub enum ClientPoll {
    Event(ClientEvent),
    Empty,
}

struct ExitState {
    sent: bool,
}

impl ExitState {
    fn new() -> Self {
        Self { sent: false }
    }

    fn on_eof(&mut self) -> ClientPoll {
        if self.sent {
            ClientPoll::Empty
        } else {
            self.sent = true;
            ClientPoll::Event(ClientEvent::Exited)
        }
    }
}

struct LineBuffer {
    bytes: Vec<u8>,
}

enum LineRead {
    Line(String),
    Empty,
}

impl LineBuffer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn append(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }

    fn next_line(&mut self) -> io::Result<LineRead> {
        let mut idx = 0;
        while idx < self.bytes.len() {
            if self.bytes[idx] == b'\n' {
                let mut line_bytes: Vec<u8> = self.bytes.drain(..=idx).collect();
                if let Some(last) = line_bytes.last() {
                    if *last == b'\n' {
                        line_bytes.pop();
                    }
                }
                if let Some(last) = line_bytes.last() {
                    if *last == b'\r' {
                        line_bytes.pop();
                    }
                }
                let line = match String::from_utf8(line_bytes) {
                    Ok(line) => line,
                    Err(err) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, err));
                    }
                };
                return Ok(LineRead::Line(line));
            }
            idx += 1;
        }
        Ok(LineRead::Empty)
    }
}

struct NonBlockingStdout {
    stdout: ChildStdout,
}

impl NonBlockingStdout {
    fn new(stdout: ChildStdout) -> io::Result<Self> {
        let wrapper = Self { stdout };
        wrapper.configure_non_blocking()?;
        Ok(wrapper)
    }

    fn read_chunk(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stdout.read(buf)
    }

    fn configure_non_blocking(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.stdout.as_raw_fd();
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }
            let set_flags = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
            if set_flags < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

pub struct RpcClient {
    child: Child,
    stdin: ChildStdin,
    stdout: NonBlockingStdout,
    line_buffer: LineBuffer,
    exit_state: ExitState,
}

impl RpcClient {
    pub fn send(&mut self, request: &RpcRequest) -> io::Result<()> {
        serde_json::to_writer(&mut self.stdin, request)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }

    pub fn poll_event(&mut self) -> io::Result<ClientPoll> {
        let pending = self.next_event()?;
        match pending {
            ClientPoll::Event(_) => return Ok(pending),
            ClientPoll::Empty => {}
        }

        let mut buffer = [0u8; 4096];
        match self.stdout.read_chunk(&mut buffer) {
            Ok(0) => Ok(self.exit_state.on_eof()),
            Ok(count) => {
                self.line_buffer.append(&buffer[..count]);
                self.next_event()
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::WouldBlock {
                    Ok(ClientPoll::Empty)
                } else {
                    Err(err)
                }
            }
        }
    }

    fn next_event(&mut self) -> io::Result<ClientPoll> {
        match self.line_buffer.next_line()? {
            LineRead::Line(line) => self.parse_line(line),
            LineRead::Empty => Ok(ClientPoll::Empty),
        }
    }

    fn parse_line(&mut self, line: String) -> io::Result<ClientPoll> {
        if line.trim().is_empty() {
            return Ok(ClientPoll::Empty);
        }
        match serde_json::from_str::<RpcResponse>(&line) {
            Ok(resp) => Ok(ClientPoll::Event(ClientEvent::Response(resp))),
            Err(err) => Ok(ClientPoll::Event(ClientEvent::Response(
                RpcResponse::Error {
                    message: format!("invalid response: {}", err),
                },
            ))),
        }
    }
}

pub enum ClientFilePath {
    Provided(String),
    Missing,
}

impl ClientFilePath {
    pub fn launcher(self) -> RpcClientLauncher {
        RpcClientLauncher { target: self }
    }

    fn apply(&self, cmd: &mut Command) {
        match self {
            ClientFilePath::Provided(path) => {
                cmd.arg(path);
            }
            ClientFilePath::Missing => {}
        }
    }
}

pub struct RpcClientLauncher {
    target: ClientFilePath,
}

impl RpcClientLauncher {
    pub fn launch(self) -> io::Result<RpcClient> {
        let current_exe = std::env::current_exe()?;
        let core_exe =
            current_exe.with_file_name(format!("vimrust-core{}", std::env::consts::EXE_SUFFIX));
        let mut cmd = Command::new(core_exe);
        self.target.apply(&mut cmd);
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => return Err(MissingPipe { name: "stdout" }.to_error()),
        };
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => return Err(MissingPipe { name: "stdin" }.to_error()),
        };

        Ok(RpcClient {
            child,
            stdin,
            stdout: NonBlockingStdout::new(stdout)?,
            line_buffer: LineBuffer::new(),
            exit_state: ExitState::new(),
        })
    }
}
