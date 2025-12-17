use std::io::{self, BufRead, Write};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{EditorMode, editor::Editor, file::File};

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
/// - ack: {"type":"ack","kind":"save"|"save_as"|"open","message":"...","file_path":"/tmp/foo.txt"} for success confirmation
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
                let frame = build_frame(&editor, &mode, &status_message, size, None);
                if let Err(err) = serde_json::to_writer(&mut stdout, &RpcResponse::Frame(frame)) {
                    let _ = write_error(&mut stdout, format!("failed to serialize frame: {}", err));
                } else {
                    let _ = stdout.write_all(b"\n");
                    let _ = stdout.flush();
                }
            }
            RequestOutcome::Ack(ack) => {
                if let Err(err) = write_ack(&mut stdout, ack) {
                    let _ = write_error(&mut stdout, format!("failed to serialize ack: {}", err));
                }
            }
            RequestOutcome::FrameAndAck(ack) => {
                if let Err(err) = write_ack(&mut stdout, ack) {
                    let _ = write_error(&mut stdout, format!("failed to serialize ack: {}", err));
                    continue;
                }
                let frame = build_frame(&editor, &mode, &status_message, size, None);
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
pub enum RpcRequest {
    Resize {
        cols: u16,
        rows: u16,
        suppress_frame: bool,
    },
    Open {
        path: String,
    },
    Save,
    SaveAs {
        path: String,
    },
    Insert {
        text: String,
    },
    Delete {
        kind: DeleteKind,
    },
    MoveCursor {
        direction: MoveDir,
    },
    SetMode {
        mode: RpcMode,
    },
    GetState,
    Quit,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteKind {
    Backspace,
    Under,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveDir {
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
pub enum RpcMode {
    Normal,
    Edit,
    Command,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RpcResponse {
    Frame(Frame),
    Ack(Ack),
    Error { message: String },
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Ack {
    pub kind: AckKind,
    pub message: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AckKind {
    Open,
    Save,
    SaveAs,
}

#[derive(Serialize)]
pub struct Frame {
    pub mode: &'static str,
    pub cursor: Cursor,
    pub rows: Vec<String>,
    pub status: Option<String>,
    pub file_path: Option<String>,
    pub size: (u16, u16),
    pub command_ui: Option<CommandUiFrame>,
}

#[derive(Serialize)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
}

#[derive(Serialize, Clone)]
pub struct CommandUiFrame {
    pub line: String,
    pub cursor_x: u16,
    pub focus_on_list: bool,
    pub list_items: Vec<CommandListItemFrame>,
    pub selected_index: Option<usize>,
    pub scroll_offset: usize,
}

#[derive(Serialize, Clone)]
pub struct CommandListItemFrame {
    pub name: String,
    pub description: String,
}

pub enum RequestOutcome {
    Frame,
    Ack(Ack),
    FrameAndAck(Ack),
    Skip,
    Quit,
    Error(String),
}

pub fn handle_request(
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
            if suppress_frame {
                RequestOutcome::Skip
            } else if *size == prev {
                RequestOutcome::Skip
            } else {
                RequestOutcome::Frame
            }
        }
        RpcRequest::Open { path } => {
            let previous_path = editor.file.path().cloned();
            let mut new_file = File::new(Some(path));
            if let Err(err) = new_file.read() {
                return RequestOutcome::Error(format!("open failed: {}", err));
            }
            *editor = Editor::new(new_file);
            *status = None;
            *mode = EditorMode::Normal;
            let ack = Ack {
                kind: AckKind::Open,
                message: Some(String::from("opened")),
                file_path: editor.file.path().cloned().or(previous_path),
            };
            RequestOutcome::FrameAndAck(ack)
        }
        RpcRequest::Save => {
            let previous_path = editor.file.path().cloned();
            match editor.file_save(&mut None) {
                Ok(msg) => {
                    *status = Some(msg.clone());
                    let ack = Ack {
                        kind: AckKind::Save,
                        message: Some(msg),
                        file_path: editor.file.path().cloned(),
                    };
                    if editor.file.path() != previous_path.as_ref() {
                        RequestOutcome::FrameAndAck(ack)
                    } else {
                        RequestOutcome::Ack(ack)
                    }
                }
                Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
            }
        }
        RpcRequest::SaveAs { path } => {
            let mut new_file = File::new(Some(path));
            new_file.file_lines = editor.file.file_lines.clone();
            match new_file.save() {
                Ok(msg) => {
                    *editor = Editor::new(new_file);
                    *status = Some(msg.clone());
                    let ack = Ack {
                        kind: AckKind::SaveAs,
                        message: Some(msg),
                        file_path: editor.file.path().cloned(),
                    };
                    RequestOutcome::FrameAndAck(ack)
                }
                Err(err) => RequestOutcome::Error(format!("save_as failed: {}", err)),
            }
        }
        RpcRequest::Insert { text } => {
            let mut changed = false;
            for ch in text.chars() {
                if editor.char_insert(ch) {
                    changed = true;
                }
            }
            if changed {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::Delete { kind } => {
            let changed = match kind {
                DeleteKind::Backspace => editor.backspace_delete(),
                DeleteKind::Under => editor.under_cursor_delete(),
            };
            if changed {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::MoveCursor { direction } => {
            let usable_rows = size.1.saturating_sub(2);
            let moved = match direction {
                MoveDir::Left => editor.cursor_move(KeyCode::Char('h'), usable_rows),
                MoveDir::Right => editor.cursor_move(KeyCode::Char('l'), usable_rows),
                MoveDir::Up => editor.cursor_move(KeyCode::Char('k'), usable_rows),
                MoveDir::Down => editor.cursor_move(KeyCode::Char('j'), usable_rows),
                MoveDir::PageUp => editor.cursor_move(KeyCode::PageUp, usable_rows),
                MoveDir::PageDown => editor.cursor_move(KeyCode::PageDown, usable_rows),
                MoveDir::Home => editor.cursor_move(KeyCode::Home, usable_rows),
                MoveDir::End => editor.cursor_move(KeyCode::End, usable_rows),
            };
            if moved {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::SetMode { mode: new_mode } => {
            let prev_mode = *mode;
            let prev_cursor = (editor.cursor_x, editor.cursor_y);
            *mode = match new_mode {
                RpcMode::Normal => EditorMode::Normal,
                RpcMode::Edit => {
                    editor.snap_cursor_to_tab_start();
                    EditorMode::Edit
                }
                RpcMode::Command => EditorMode::Command,
            };
            if *mode != prev_mode || (editor.cursor_x, editor.cursor_y) != prev_cursor {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::GetState => RequestOutcome::Frame,
        RpcRequest::Quit => RequestOutcome::Quit,
    }
}

pub fn build_frame(
    editor: &Editor,
    mode: &EditorMode,
    status: &Option<String>,
    size: (u16, u16),
    command_ui: Option<CommandUiFrame>,
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
    let cursor_row = base_row.max(1).min(size.1.saturating_sub(1).max(1));

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
        command_ui,
    }
}

fn write_error(stdout: &mut impl Write, message: String) -> io::Result<()> {
    serde_json::to_writer(&mut *stdout, &RpcResponse::Error { message })?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn write_ack(stdout: &mut impl Write, ack: Ack) -> io::Result<()> {
    serde_json::to_writer(&mut *stdout, &RpcResponse::Ack(ack))?;
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

        let frame = build_frame(&editor, &mode, &status, size, None);
        assert_eq!(frame.rows.get(0).map(String::as_str), Some("hi"));
    }

    #[test]
    fn noop_cursor_move_skips_frame() {
        let mut editor = Editor::new(File::new(None));
        editor.file.file_lines = vec![String::new()];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);

        let outcome = handle_request(
            RpcRequest::MoveCursor {
                direction: MoveDir::Left,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn resize_with_suppress_frame_skips() {
        let mut editor = Editor::new(File::new(None));
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);

        let outcome = handle_request(
            RpcRequest::Resize {
                cols: 20,
                rows: 40,
                suppress_frame: true,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert_eq!(size, (20, 40));
    }

    #[test]
    fn noop_delete_under_cursor_skips_frame() {
        let mut editor = Editor::new(File::new(None));
        editor.file.file_lines = vec![String::new()];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);

        let outcome = handle_request(
            RpcRequest::Delete {
                kind: DeleteKind::Under,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
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
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind, AckKind::SaveAs);
            assert_eq!(ack.message.as_deref(), Some("written"));
            assert_eq!(
                ack.file_path.as_deref(),
                Some(path.to_string_lossy().as_ref())
            );
        } else {
            panic!("expected frame and ack");
        }
        assert_eq!(status.as_deref(), Some("written"));
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "hello");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn save_on_existing_path_emits_ack_without_frame() {
        let path = std::env::temp_dir().join("vimrust_rpc_save_exists.txt");
        let _ = fs::write(&path, "existing");

        let mut editor = Editor::new(File::new(Some(
            path.to_string_lossy().to_string(),
        )));
        editor.file.file_lines = vec![String::from("changed")];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);

        let outcome = handle_request(
            RpcRequest::Save,
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
        );
        if let RequestOutcome::Ack(ack) = outcome {
            assert_eq!(ack.kind, AckKind::Save);
            assert_eq!(ack.message.as_deref(), Some("written"));
            assert_eq!(
                ack.file_path.as_deref(),
                Some(path.to_string_lossy().as_ref())
            );
        } else {
            panic!("expected ack without frame");
        }
        assert_eq!(status.as_deref(), Some("written"));
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "changed");
        let _ = fs::remove_file(&path);
    }
}
