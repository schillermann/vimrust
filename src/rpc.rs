use std::io::{self, BufRead, Write};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{editor::Editor, file::File, EditorMode};

/// Line-delimited JSON RPC loop for driving the editor core without the terminal UI.
///
/// Requests (`"type"` field):
/// - resize: {"type":"resize","cols":80,"rows":24}
/// - open: {"type":"open","path":"/tmp/file.txt"}
/// - save: {"type":"save"}
/// - save_as: {"type":"save_as","path":"/tmp/new.txt"}
/// - insert: {"type":"insert","text":"hello"}
/// - delete: {"type":"delete","kind":"backspace"|"under"}
/// - move_cursor: {"type":"move_cursor","direction":"left"|"right"|"up"|"down"|"page_up"|"page_down"|"home"|"end"}
/// - set_mode: {"type":"set_mode","mode":"normal"|"edit"|"command"}
/// - get_state: {"type":"get_state"}
/// - quit: {"type":"quit"}
///
/// Responses:
/// - frame: {"type":"frame", ...} for state snapshots (emitted after state changes and on get_state)
/// - error: {"type":"error","message":"..."} on failure
pub fn serve_stdio(file_path: Option<String>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    let file = File::new(file_path.clone());
    let mut editor = Editor::new(file);
    editor.file_read()?;
    let mut mode = EditorMode::Normal;
    let mut status_message: Option<String> = None;
    let mut size: (u16, u16) = (80, 24);

    while let Some(line) = lines.next() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: RpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(err) => {
                let _ = write_error(&mut stdout, format!("invalid JSON: {}", err));
                continue;
            }
        };

        match handle_request(
            request,
            &mut editor,
            &mut mode,
            &mut status_message,
            &mut size,
        ) {
            RequestOutcome::Frame => {
                let frame = build_frame(&editor, &mode, &status_message, size);
                if let Err(err) = serde_json::to_writer(&mut stdout, &RpcResponse::Frame(frame)) {
                    let _ = write_error(&mut stdout, format!("failed to serialize frame: {}", err));
                } else {
                    let _ = stdout.write_all(b"\n");
                    let _ = stdout.flush();
                }
            }
            RequestOutcome::Quit => break,
            RequestOutcome::Skip => {}
            RequestOutcome::Error(message) => {
                let _ = write_error(&mut stdout, message);
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RpcRequest {
    Resize {
        cols: u16,
        rows: u16,
        suppress_frame: bool,
    },
    Open { path: String },
    Save,
    SaveAs { path: String },
    Insert { text: String },
    Delete { kind: DeleteKind },
    MoveCursor { direction: MoveDir },
    SetMode { mode: RpcMode },
    GetState,
    Quit,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeleteKind {
    Backspace,
    Under,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MoveDir {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum RpcMode {
    Normal,
    Edit,
    Command,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RpcResponse {
    Frame(Frame),
    Error { message: String },
}

#[derive(Serialize)]
struct Frame {
    mode: &'static str,
    cursor: Cursor,
    rows: Vec<String>,
    status: Option<String>,
    file_path: Option<String>,
    size: (u16, u16),
}

#[derive(Serialize)]
struct Cursor {
    col: u16,
    row: u16,
}

enum RequestOutcome {
    Frame,
    Skip,
    Quit,
    Error(String),
}

fn handle_request(
    request: RpcRequest,
    editor: &mut Editor,
    mode: &mut EditorMode,
    status: &mut Option<String>,
    size: &mut (u16, u16),
) -> RequestOutcome {
    match request {
        RpcRequest::Resize {
            cols,
            rows,
            suppress_frame,
        } => {
            let prev = *size;
            *size = (cols, rows);
            if suppress_frame && *size == prev {
                RequestOutcome::Skip
            } else {
                RequestOutcome::Frame
            }
        }
        RpcRequest::Open { path } => {
            let mut new_file = File::new(Some(path));
            if let Err(err) = new_file.read() {
                return RequestOutcome::Error(format!("open failed: {}", err));
            }
            *editor = Editor::new(new_file);
            *status = None;
            *mode = EditorMode::Normal;
            RequestOutcome::Frame
        }
        RpcRequest::Save => match editor.save(&mut None) {
            Ok(msg) => {
                *status = Some(msg);
                RequestOutcome::Frame
            }
            Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
        },
        RpcRequest::SaveAs { path } => {
            let mut new_file = File::new(Some(path));
            new_file.file_lines = editor.file.file_lines.clone();
            match new_file.save() {
                Ok(msg) => {
                    *editor = Editor::new(new_file);
                    *status = Some(msg);
                    RequestOutcome::Frame
                }
                Err(err) => RequestOutcome::Error(format!("save_as failed: {}", err)),
            }
        }
        RpcRequest::Insert { text } => {
            for ch in text.chars() {
                editor.insert_char(ch);
            }
            RequestOutcome::Frame
        }
        RpcRequest::Delete { kind } => {
            match kind {
                DeleteKind::Backspace => editor.delete_backspace(),
                DeleteKind::Under => editor.delete_under_cursor(),
            }
            RequestOutcome::Frame
        }
        RpcRequest::MoveCursor { direction } => {
            let usable_rows = size.1.saturating_sub(2);
            match direction {
                MoveDir::Left => editor.cursor_move(KeyCode::Char('h'), usable_rows),
                MoveDir::Right => editor.cursor_move(KeyCode::Char('l'), usable_rows),
                MoveDir::Up => editor.cursor_move(KeyCode::Char('k'), usable_rows),
                MoveDir::Down => editor.cursor_move(KeyCode::Char('j'), usable_rows),
                MoveDir::PageUp => editor.cursor_move(KeyCode::PageUp, usable_rows),
                MoveDir::PageDown => editor.cursor_move(KeyCode::PageDown, usable_rows),
                MoveDir::Home => editor.cursor_move(KeyCode::Home, usable_rows),
                MoveDir::End => editor.cursor_move(KeyCode::End, usable_rows),
            }
            RequestOutcome::Frame
        }
        RpcRequest::SetMode { mode: new_mode } => {
            *mode = match new_mode {
                RpcMode::Normal => EditorMode::Normal,
                RpcMode::Edit => {
                    editor.snap_cursor_to_tab_start();
                    EditorMode::Edit
                }
                RpcMode::Command => EditorMode::Command,
            };
            RequestOutcome::Frame
        }
        RpcRequest::GetState => RequestOutcome::Frame,
        RpcRequest::Quit => RequestOutcome::Quit,
    }
}

fn build_frame(
    editor: &Editor,
    mode: &EditorMode,
    status: &Option<String>,
    size: (u16, u16),
) -> Frame {
    let usable_rows = size.1.saturating_sub(2);
    let view = editor.view_with_scroll(size.0, usable_rows);
    let rows = editor.rows_render(&view, size.0, usable_rows);

    let cursor_col = view
        .cursor_x
        .saturating_sub(view.columns_offset)
        .min(size.0.saturating_sub(1));
    let base_row = view
        .cursor_y
        .saturating_sub(view.rows_offset)
        .saturating_add(1);
    let cursor_row = base_row
        .max(1)
        .min(size.1.saturating_sub(1).max(1));

    let mut status_text = format!(
        "{} > {}",
        mode.label(),
        view.file
            .path()
            .cloned()
            .unwrap_or_else(|| "[No Filename]".into())
    );
    if let Some(msg) = status {
        if !msg.is_empty() {
            status_text.push_str(" > ");
            status_text.push_str(msg);
        }
    }
    let status = Some(status_text);

    Frame {
        mode: mode.label(),
        cursor: Cursor {
            col: cursor_col,
            row: cursor_row,
        },
        rows,
        status,
        file_path: view.file.path().cloned(),
        size,
    }
}

fn write_error(stdout: &mut impl Write, message: String) -> io::Result<()> {
    serde_json::to_writer(&mut *stdout, &RpcResponse::Error { message })?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn insert_request_updates_rows() {
        let mut editor = Editor::new(File::new(None));
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);

        let outcome = handle_request(
            RpcRequest::Insert {
                text: "hi".to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        assert!(matches!(outcome, RequestOutcome::Frame));

        let frame = build_frame(&editor, &mode, &status, size);
        assert_eq!(frame.rows.get(0).map(String::as_str), Some("hi"));
    }

    #[test]
    fn save_as_writes_file_and_updates_status() {
        let mut editor = Editor::new(File::new(None));
        editor.file.file_lines = vec![String::from("hello")];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);
        let path = std::env::temp_dir().join("vimrust_rpc_test.txt");
        let _ = fs::remove_file(&path);

        let outcome = handle_request(
            RpcRequest::SaveAs {
                path: path.to_string_lossy().to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        assert!(matches!(outcome, RequestOutcome::Frame));
        assert_eq!(status.as_deref(), Some("written"));
        assert_eq!(
            fs::read_to_string(&path).unwrap_or_default(),
            String::from("hello")
        );
        let _ = fs::remove_file(&path);
    }
}
