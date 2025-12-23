use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use vimrust_protocol::{AckKind, RpcRequest, RpcResponse};

fn read_response(reader: &mut BufReader<std::process::ChildStdout>) -> RpcResponse {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read response");
    assert!(!line.trim().is_empty(), "empty response line");
    serde_json::from_str(&line).expect("parse response")
}

#[test]
fn rpc_stdio_end_to_end_frame_ack_error() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vimrust-core"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn core");

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let state_get = RpcRequest::StateGet;
    serde_json::to_writer(&mut stdin, &state_get).expect("write state_get");
    stdin.write_all(b"\n").expect("newline");
    stdin.flush().expect("flush");

    let response = read_response(&mut reader);
    match response {
        RpcResponse::Frame(frame) => {
            assert_eq!(frame.size(), (80, 24));
        }
        _ => panic!("expected frame"),
    }

    let file_save = RpcRequest::FileSave;
    serde_json::to_writer(&mut stdin, &file_save).expect("write file_save");
    stdin.write_all(b"\n").expect("newline");
    stdin.flush().expect("flush");

    let response = read_response(&mut reader);
    match response {
        RpcResponse::Ack(ack) => {
            assert_eq!(ack.kind(), AckKind::Save);
        }
        _ => panic!("expected ack"),
    }

    // Consume the frame emitted after the ack.
    let response = read_response(&mut reader);
    assert!(matches!(response, RpcResponse::Frame(_)));

    stdin.write_all(b"not json\n").expect("write invalid json");
    stdin.flush().expect("flush");

    let response = read_response(&mut reader);
    match response {
        RpcResponse::Error { message } => {
            assert!(message.starts_with("invalid JSON:"));
        }
        _ => panic!("expected error"),
    }

    let _ = child.kill();
}
