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

pub trait ClientEventHandler {
    fn accept(&mut self, event: ClientEvent) -> io::Result<()>;
}

struct ExitState {
    sent: bool,
}

impl ExitState {
    fn new() -> Self {
        Self { sent: false }
    }

    fn emit(&mut self, handler: &mut dyn ClientEventHandler) -> io::Result<()> {
        if self.sent {
            return Ok(());
        }
        self.sent = true;
        handler.accept(ClientEvent::Exited)
    }
}

struct LineBuffer {
    bytes: Vec<u8>,
}

impl LineBuffer {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn append(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }

    fn emit(&mut self, handler: &mut dyn LineHandler) -> io::Result<()> {
        loop {
            let mut newline_index = self.bytes.len();
            let mut idx = 0;
            while idx < self.bytes.len() {
                if self.bytes[idx] == b'\n' {
                    newline_index = idx;
                    break;
                }
                idx = idx.saturating_add(1);
            }
            if newline_index == self.bytes.len() {
                return Ok(());
            }
            let mut line_bytes: Vec<u8> = self.bytes.drain(..=newline_index).collect();
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
            handler.accept(line)?;
        }
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

    fn read_chunk(&mut self, buf: &mut [u8], chunk: &mut ReadChunk) -> io::Result<()> {
        let count = self.stdout.read(buf)?;
        chunk.record(count);
        Ok(())
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

    pub fn accept(&mut self, handler: &mut dyn ClientEventHandler) -> io::Result<()> {
        loop {
            let parser = ResponseLineParser;
            let mut line_handler = ClientLineHandler::new(&parser, handler);
            self.line_buffer.emit(&mut line_handler)?;

            let mut buffer = [0u8; 4096];
            let mut chunk = ReadChunk::new();
            match self.stdout.read_chunk(&mut buffer, &mut chunk) {
                Ok(()) => {
                    let count = chunk.count();
                    if count == 0 {
                        self.exit_state.emit(handler)?;
                        return Ok(());
                    }
                    self.line_buffer.append(&buffer[..count]);
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::WouldBlock {
                        return Ok(());
                    }
                    return Err(err);
                }
            }
        }
    }
}

trait LineHandler {
    fn accept(&mut self, line: String) -> io::Result<()>;
}

struct ClientLineHandler<'a> {
    parser: &'a ResponseLineParser,
    handler: &'a mut dyn ClientEventHandler,
}

impl<'a> ClientLineHandler<'a> {
    fn new(parser: &'a ResponseLineParser, handler: &'a mut dyn ClientEventHandler) -> Self {
        Self { parser, handler }
    }
}

impl<'a> LineHandler for ClientLineHandler<'a> {
    fn accept(&mut self, line: String) -> io::Result<()> {
        self.parser.accept(line, self.handler)
    }
}

struct ResponseLineParser;

impl ResponseLineParser {
    fn accept(&self, line: String, handler: &mut dyn ClientEventHandler) -> io::Result<()> {
        if line.trim().is_empty() {
            return Ok(());
        }
        match serde_json::from_str::<RpcResponse>(&line) {
            Ok(resp) => handler.accept(ClientEvent::Response(resp)),
            Err(err) => handler.accept(ClientEvent::Response(RpcResponse::Error {
                message: format!("invalid response: {}", err),
            })),
        }
    }
}

struct ReadChunk {
    count: usize,
}

impl ReadChunk {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn record(&mut self, count: usize) {
        self.count = count;
    }

    fn count(&self) -> usize {
        self.count
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
