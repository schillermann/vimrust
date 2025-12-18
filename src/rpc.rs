use std::io::{self, BufRead, Write};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{
    EditorMode,
    command_ui_state::{CommandUiAction, CommandUiState},
    editor::Editor,
    file::File,
};

pub use crate::command_ui_state::CommandUiFrame;

/// Line-delimited JSON RPC loop for driving the editor core without the terminal UI.
///
/// Requests (`"type"` field):
/// - editor_resize: {"type":"editor_resize","cols":80,"rows":24}
/// - file_open: {"type":"file_open","path":"/tmp/file.txt"}
/// - file_save: {"type":"file_save"}
/// - file_save_as: {"type":"file_save_as","path":"/tmp/new.txt"}
/// - text_insert: {"type":"text_insert","text":"hello"}
/// - text_delete: {"type":"text_delete","kind":"backspace"|"under"}
/// - cursor_move: {"type":"cursor_move","direction":"left"|"right"|"up"|"down"|"page_up"|"page_down"|"home"|"end"}
/// - command_ui: {"type":"command_ui","action":"insert_char","ch":"a"} (for command-line editing/navigation)
/// - mode_set: {"type":"mode_set","mode":"normal"|"edit"|"command"}
/// - state_get: {"type":"state_get"}
/// - editor_quit: {"type":"editor_quit"}
/// - command_execute: {"type":"command_execute","line":":s"} (execute entered command text)
///
/// Responses:
/// - frame: {"type":"frame", ...} for state snapshots (emitted after state changes and on get_state)
/// - ack: {"type":"ack","kind":"save"|"save_as"|"open","message":"...","file_path":"/tmp/foo.txt"} for success confirmation
/// - error: {"type":"error","message":"..."} on failure
pub fn serve_stdio(file_path: Option<String>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let lines = stdin.lock().lines();

    let file = File::new(file_path.clone());
    let mut editor = Editor::new(file);
    editor.file_read()?;
    let mut mode = EditorMode::Normal;
    let mut status_message: Option<String> = None;
    let mut size: (u16, u16) = (80, 24);
    let mut command_ui = CommandUiState::new();

    for line in lines {
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
            &mut command_ui,
        ) {
            RequestOutcome::Frame => {
                let frame = build_frame(
                    &editor,
                    &mode,
                    &status_message,
                    size,
                    if matches!(mode, EditorMode::Command) {
                        Some(command_ui.frame())
                    } else {
                        None
                    },
                );
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
                let frame = build_frame(
                    &editor,
                    &mode,
                    &status_message,
                    size,
                    if matches!(mode, EditorMode::Command) {
                        Some(command_ui.frame())
                    } else {
                        None
                    },
                );
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
    EditorResize {
        cols: u16,
        rows: u16,
        suppress_frame: bool,
    },
    FileOpen {
        path: String,
    },
    FileSave,
    FileSaveAs {
        path: String,
    },
    TextInsert {
        text: String,
    },
    TextDelete {
        kind: DeleteKind,
    },
    CursorMove {
        direction: MoveDir,
    },
    CommandUi {
        action: CommandUiAction,
    },
    ModeSet {
        mode: RpcMode,
    },
    StateGet,
    EditorQuit,
    CommandExecute {
        line: String,
    },
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
    command_ui: &mut CommandUiState,
) -> RequestOutcome {
    match request {
        RpcRequest::EditorResize {
            cols,
            rows,
            suppress_frame,
        } => {
            let prev = *size;
            *size = (cols, rows);
            if matches!(mode, EditorMode::Command) {
                let list_rows = command_list_rows(*size);
                command_ui
                    .command_list
                    .adjust_scroll_for_visible_rows(list_rows);
            }
            if suppress_frame || *size == prev {
                RequestOutcome::Skip
            } else {
                RequestOutcome::Frame
            }
        }
        RpcRequest::FileOpen { path } => {
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
        RpcRequest::FileSave => {
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
        RpcRequest::FileSaveAs { path } => {
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
        RpcRequest::TextInsert { text } => {
            let mut changed = false;
            for ch in text.chars() {
                if editor.char_insert(ch) {
                    changed = true;
                }
            }
            if changed {
                *status = Some(String::from("modified"));
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::TextDelete { kind } => {
            let changed = match kind {
                DeleteKind::Backspace => editor.backspace_delete(),
                DeleteKind::Under => editor.under_cursor_delete(),
            };
            if changed {
                *status = Some(String::from("modified"));
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::CursorMove { direction } => {
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
        RpcRequest::CommandUi { action } => {
            if !matches!(mode, EditorMode::Command) {
                return RequestOutcome::Skip;
            }
            let list_rows = command_list_rows(*size);
            let changed = command_ui.apply_action(action, list_rows);
            if changed {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::CommandExecute { line } => {
            if !matches!(mode, EditorMode::Command) {
                return RequestOutcome::Skip;
            }
            let command = line.trim_start_matches(':').trim().to_lowercase();
            match command.as_str() {
                "s" | "save" => match editor.file_save(&mut None) {
                    Ok(msg) => {
                        *status = Some(msg.clone());
                        if matches!(mode, EditorMode::Command) {
                            command_ui.clear();
                        }
                        *mode = EditorMode::Normal;
                        let ack = Ack {
                            kind: AckKind::Save,
                            message: Some(msg),
                            file_path: editor.file.path().cloned(),
                        };
                        RequestOutcome::FrameAndAck(ack)
                    }
                    Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                },
                "sq" => match editor.file_save(&mut None) {
                    Ok(msg) => {
                        *status = Some(msg.clone());
                        RequestOutcome::Quit
                    }
                    Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                },
                "q" | "quit" => RequestOutcome::Quit,
                _ => RequestOutcome::Skip,
            }
        }
        RpcRequest::ModeSet { mode: new_mode } => {
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
            if *mode == EditorMode::Command && prev_mode != EditorMode::Command {
                command_ui.start_prompt();
            } else if prev_mode == EditorMode::Command && *mode != EditorMode::Command {
                command_ui.clear();
            }
            if *mode != prev_mode || (editor.cursor_x, editor.cursor_y) != prev_cursor {
                RequestOutcome::Frame
            } else {
                RequestOutcome::Skip
            }
        }
        RpcRequest::StateGet => RequestOutcome::Frame,
        RpcRequest::EditorQuit => RequestOutcome::Quit,
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

    let status = if editor.file_changed() {
        Some(String::from("modified"))
    } else {
        status.clone()
    };

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

fn command_list_rows(size: (u16, u16)) -> usize {
    size.1
        .saturating_sub(2) // command line + status line
        .saturating_sub(3) as usize // blank + header + divider rows
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
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::TextInsert {
                text: "hi".to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
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
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CursorMove {
                direction: MoveDir::Left,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn resize_with_suppress_frame_skips() {
        let mut editor = Editor::new(File::new(None));
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::EditorResize {
                cols: 20,
                rows: 40,
                suppress_frame: true,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert_eq!(size, (20, 40));
    }

    #[test]
    fn command_ui_request_updates_state_and_frame() {
        let mut editor = Editor::new(File::new(None));
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (20, 8);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::ModeSet {
                mode: RpcMode::Command,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Frame));

        let outcome = handle_request(
            RpcRequest::CommandUi {
                action: CommandUiAction::InsertChar { ch: 'x' },
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Frame));

        let frame = build_frame(&editor, &mode, &status, size, Some(command_ui.frame()));
        let command_ui_frame = frame.command_ui.unwrap();
        assert_eq!(command_ui_frame.line, ":x");
        assert_eq!(command_ui_frame.cursor_x, 2);
    }

    #[test]
    fn noop_delete_under_cursor_skips_frame() {
        let mut editor = Editor::new(File::new(None));
        editor.file.file_lines = vec![String::new()];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::TextDelete {
                kind: DeleteKind::Under,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
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
        let mut command_ui = CommandUiState::new();
        let path = std::env::temp_dir().join("vimrust_rpc_test.txt");
        let _ = fs::remove_file(&path);

        let outcome = handle_request(
            RpcRequest::FileSaveAs {
                path: path.to_string_lossy().to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
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
    fn command_execute_save_exits_command_and_emits_ack() {
        let path = std::env::temp_dir().join("vimrust_rpc_command_save.txt");
        let _ = fs::remove_file(&path);

        let mut editor = Editor::new(File::new(Some(path.to_string_lossy().to_string())));
        editor.file.file_lines = vec![String::from("changed")];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let _ = handle_request(
            RpcRequest::ModeSet {
                mode: RpcMode::Command,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        let outcome = handle_request(
            RpcRequest::CommandExecute {
                line: ":s".to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind, AckKind::Save);
            assert!(matches!(mode, EditorMode::Normal));
            assert_eq!(status.as_deref(), Some("written"));
        } else {
            panic!("expected frame and ack");
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_quit_requests_quit_outcome() {
        let mut editor = Editor::new(File::new(None));
        let mut mode = EditorMode::Command;
        let mut status = None;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CommandExecute {
                line: ":q".to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Quit));
    }

    #[test]
    fn save_on_existing_path_emits_ack_without_frame() {
        let path = std::env::temp_dir().join("vimrust_rpc_save_exists.txt");
        let _ = fs::write(&path, "existing");

        let mut editor = Editor::new(File::new(Some(path.to_string_lossy().to_string())));
        editor.file.file_lines = vec![String::from("changed")];
        let mut mode = EditorMode::Normal;
        let mut status = None;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::FileSave,
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
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
