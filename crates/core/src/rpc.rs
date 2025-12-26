use std::io::{self, BufRead, Write};

use crate::{
    EditorMode, FrameSignal, command_ui_state::CommandUiState, editor::Editor, file::File,
};
use vimrust_protocol::{
    Ack, AckKind, CommandUiAction, CommandUiFrame, Cursor, DeleteKind, FilePath, Frame,
    ProtocolVersion, RpcMode, RpcRequest, RpcResponse, StatusMessage,
};

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
/// - command_execute: {"type":"command_execute","line":":s"} (execute entered command text; if
///   omitted, the current command line stored in the core is used)
///
/// Responses:
/// - frame: {"type":"frame", ...} for state snapshots (emitted after state changes and on get_state)
/// - ack: {"type":"ack","kind":"save"|"save_as"|"open","message":{"kind":"text","text":"..."},"file_path":{"kind":"provided","path":"/tmp/foo.txt"}} for success confirmation
/// - error: {"type":"error","message":"..."} on failure
pub fn serve_stdio(file_path: FilePath) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let lines = stdin.lock().lines();

    let file = File::new(file_path.clone());
    let mut editor = Editor::new(file);
    editor.file_read()?;
    let mut mode = EditorMode::Normal;
    let mut status_message = StatusMessage::Empty;
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
    status: &mut StatusMessage,
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
                command_ui.list_scroll_adjust(list_rows);
            }
            if suppress_frame || *size == prev {
                RequestOutcome::Skip
            } else {
                RequestOutcome::Frame
            }
        }
        RpcRequest::FileOpen { path } => {
            let mut new_file = File::new(FilePath::Provided { path });
            if let Err(err) = new_file.read() {
                return RequestOutcome::Error(format!("open failed: {}", err));
            }
            *editor = Editor::new(new_file);
            *status = StatusMessage::Empty;
            *mode = EditorMode::Normal;
            let ack = Ack::new(
                AckKind::Open,
                StatusMessage::Text {
                    text: String::from("opened"),
                },
                editor.file_path(),
            );
            RequestOutcome::FrameAndAck(ack)
        }
        RpcRequest::FileSave => {
            let previous_path = editor.file_path();
            let mut saved_path = FilePath::Missing;
            match editor.file_save(&mut saved_path) {
                Ok(msg) => {
                    *status = StatusMessage::Text { text: msg.clone() };
                    let ack =
                        Ack::new(AckKind::Save, StatusMessage::Text { text: msg }, saved_path);
                    if editor.file_path() != previous_path {
                        RequestOutcome::FrameAndAck(ack)
                    } else {
                        RequestOutcome::Ack(ack)
                    }
                }
                Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
            }
        }
        RpcRequest::FileSaveAs { path } => {
            let mut new_file = File::new(FilePath::Provided { path });
            new_file.lines_replace(editor.file_lines_snapshot());
            match new_file.save() {
                Ok(msg) => {
                    *editor = Editor::new(new_file);
                    *status = StatusMessage::Text { text: msg.clone() };
                    let ack = Ack::new(
                        AckKind::SaveAs,
                        StatusMessage::Text { text: msg },
                        editor.file_path(),
                    );
                    RequestOutcome::FrameAndAck(ack)
                }
                Err(err) => RequestOutcome::Error(format!("save_as failed: {}", err)),
            }
        }
        RpcRequest::TextInsert { text } => {
            let snapshot = editor.snapshot();
            for ch in text.chars() {
                editor.char_insert(ch);
            }
            *status = snapshot.status_from(editor, status.clone());
            match snapshot.frame_signal(editor) {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            }
        }
        RpcRequest::TextDelete { kind } => {
            let snapshot = editor.snapshot();
            match kind {
                DeleteKind::Backspace => editor.backspace_delete(),
                DeleteKind::Under => editor.under_cursor_delete(),
            };
            *status = snapshot.status_from(editor, status.clone());
            match snapshot.frame_signal(editor) {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            }
        }
        RpcRequest::CursorMove { direction } => {
            let usable_rows = size.1.saturating_sub(2);
            let snapshot = editor.snapshot();
            editor.cursor_move(direction, usable_rows);
            match snapshot.frame_signal(editor) {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            }
        }
        RpcRequest::CommandUi { action } => {
            if !matches!(mode, EditorMode::Command) {
                return RequestOutcome::Skip;
            }
            let list_rows = command_list_rows(*size);
            let snapshot = command_ui.snapshot();
            command_ui.apply_action(action, list_rows);
            match snapshot.frame_signal(command_ui) {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            }
        }
        RpcRequest::CommandExecute { line } => {
            if !matches!(mode, EditorMode::Command) {
                return RequestOutcome::Skip;
            }
            let list_rows = command_list_rows(*size);
            let selection_signal = if line.is_none() {
                let snapshot = command_ui.snapshot();
                command_ui.apply_action(CommandUiAction::SelectFromList, list_rows);
                snapshot.frame_signal(command_ui)
            } else {
                FrameSignal::Skip
            };
            let source_line = match line {
                Some(line) => {
                    command_ui.line_overwrite(line.clone());
                    line
                }
                None => command_ui.command_text().to_string(),
            };
            let command = source_line.trim_start_matches(':').trim().to_lowercase();
            match command.as_str() {
                "s" | "save" => {
                    let mut saved_path = FilePath::Missing;
                    match editor.file_save(&mut saved_path) {
                        Ok(msg) => {
                            *status = StatusMessage::Text { text: msg.clone() };
                            if matches!(mode, EditorMode::Command) {
                                command_ui.clear();
                            }
                            *mode = EditorMode::Normal;
                            let ack = Ack::new(
                                AckKind::Save,
                                StatusMessage::Text { text: msg },
                                saved_path,
                            );
                            RequestOutcome::FrameAndAck(ack)
                        }
                        Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                    }
                }
                "sq" => {
                    let mut saved_path = FilePath::Missing;
                    match editor.file_save(&mut saved_path) {
                        Ok(msg) => {
                            *status = StatusMessage::Text { text: msg.clone() };
                            RequestOutcome::Quit
                        }
                        Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                    }
                }
                "q" | "quit" => RequestOutcome::Quit,
                _ => {
                    match selection_signal {
                        FrameSignal::Frame => RequestOutcome::Frame,
                        FrameSignal::Skip => RequestOutcome::Skip,
                    }
                }
            }
        }
        RpcRequest::ModeSet { mode: new_mode } => {
            let prev_mode = *mode;
            let prev_cursor = editor.cursor_position();
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
            if *mode != prev_mode || editor.cursor_position() != prev_cursor {
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
    status: &StatusMessage,
    size: (u16, u16),
    command_ui: Option<CommandUiFrame>,
) -> Frame {
    let usable_rows = size.1.saturating_sub(2);
    let view = editor.view_with_scroll(size.0, usable_rows);
    let rows = editor.rows_render(&view, size.0, usable_rows);

    let cursor_col = view
        .cursor_column()
        .saturating_sub(view.column_offset())
        .min(size.0.saturating_sub(1));
    let base_row = view
        .cursor_row()
        .saturating_sub(view.row_offset())
        .saturating_add(1);
    let cursor_row = base_row.max(1).min(size.1.saturating_sub(1).max(1));

    let status = editor.change_status().status_or(status);

    Frame::new(
        mode.label().to_string(),
        Cursor::new(cursor_col, cursor_row),
        rows,
        status,
        view.file_ref().path(),
        size,
        command_ui,
        ProtocolVersion::current(),
    )
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
    use crate::editor::CursorPosition;
    use std::fs;
    use vimrust_protocol::MoveDirection;

    #[test]
    fn insert_request_updates_rows() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
        assert_eq!(frame.editor_rows().get(0).map(String::as_str), Some("hi"));
    }

    #[test]
    fn noop_cursor_move_skips_frame() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::new()]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CursorMove {
                direction: MoveDirection::Left,
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
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
    fn resize_same_size_without_suppress_skips() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::EditorResize {
                cols: 10,
                rows: 5,
                suppress_frame: false,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert_eq!(size, (10, 5));
    }

    #[test]
    fn command_ui_request_updates_state_and_frame() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
        let command_ui_frame = frame.command_ui_frame().unwrap();
        assert_eq!(command_ui_frame.command_text(), ":x");
        assert_eq!(command_ui_frame.cursor_column(), 2);
    }

    #[test]
    fn command_ui_outside_command_mode_skips() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

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
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn noop_delete_under_cursor_skips_frame() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::new()]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
    fn file_open_missing_path_returns_error() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();
        let path = std::env::temp_dir().join("vimrust_missing_file.txt");
        let _ = fs::remove_file(&path);

        let outcome = handle_request(
            RpcRequest::FileOpen {
                path: path.to_string_lossy().to_string(),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        match outcome {
            RequestOutcome::Error(message) => {
                assert!(message.starts_with("open failed:"));
            }
            _ => panic!("expected error outcome"),
        }
    }

    #[test]
    fn save_as_writes_file_and_updates_status() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::from("hello")]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
            assert_eq!(ack.kind(), AckKind::SaveAs);
            assert_eq!(
                ack.message(),
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
            assert_eq!(
                ack.path(),
                FilePath::Provided {
                    path: path.to_string_lossy().to_string(),
                }
            );
        } else {
            panic!("expected frame and ack");
        }
        assert_eq!(
            status,
            StatusMessage::Text {
                text: String::from("saved"),
            }
        );
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "hello");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_save_exits_command_and_emits_ack() {
        let path = std::env::temp_dir().join("vimrust_rpc_command_save.txt");
        let _ = fs::remove_file(&path);

        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
                line: Some(":s".to_string()),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert!(matches!(mode, EditorMode::Normal));
            assert_eq!(
                status,
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
        } else {
            panic!("expected frame and ack");
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_without_line_uses_internal_command_ui_state() {
        let path = std::env::temp_dir().join("vimrust_rpc_command_save_no_line.txt");
        let _ = fs::remove_file(&path);

        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
        let _ = handle_request(
            RpcRequest::CommandUi {
                action: CommandUiAction::InsertChar { ch: 's' },
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        let outcome = handle_request(
            RpcRequest::CommandExecute { line: None },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert!(matches!(mode, EditorMode::Normal));
            assert_eq!(
                status,
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
        } else {
            panic!("expected frame and ack");
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_selects_list_entry_and_runs_command() {
        let path = std::env::temp_dir().join("vimrust_rpc_command_save_from_list.txt");
        let _ = fs::remove_file(&path);

        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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

        let _ = handle_request(
            RpcRequest::CommandUi {
                action: CommandUiAction::MoveSelectionDown,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        let outcome = handle_request(
            RpcRequest::CommandExecute { line: None },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert_eq!(
                status,
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
            assert!(matches!(mode, EditorMode::Normal));
        } else {
            panic!("expected frame and ack");
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_quit_requests_quit_outcome() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Command;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CommandExecute {
                line: Some(":q".to_string()),
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
    fn command_execute_unknown_command_skips() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Command;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CommandExecute {
                line: Some(":unknown".to_string()),
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert!(matches!(mode, EditorMode::Command));
    }

    #[test]
    fn command_execute_outside_command_mode_skips() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::CommandExecute {
                line: Some(":q".to_string()),
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
    fn mode_set_same_mode_skips() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = CommandUiState::new();

        let outcome = handle_request(
            RpcRequest::ModeSet {
                mode: RpcMode::Normal,
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
    fn command_ui_frame_includes_line_and_selection() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
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
        let _ = handle_request(
            RpcRequest::CommandUi {
                action: CommandUiAction::InsertChar { ch: 's' },
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        let _ = handle_request(
            RpcRequest::CommandUi {
                action: CommandUiAction::MoveSelectionDown,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        let frame = build_frame(&editor, &mode, &status, size, Some(command_ui.frame()));
        let command_ui_frame = frame.command_ui_frame().expect("expected command ui frame");
        assert_eq!(command_ui_frame.command_text(), ":s");
        assert_eq!(command_ui_frame.cursor_column(), 2);
        assert!(command_ui_frame.list_focus());
        assert!(command_ui_frame.selected_item().is_some());
        assert!(!command_ui_frame.command_items().is_empty());
    }

    #[test]
    fn frame_cursor_positions_respect_offsets() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![
            String::from("aaa"),
            String::from("bbb"),
            String::from("ccc"),
            String::from("ddd"),
        ]);
        editor.cursor_position_store(CursorPosition::new(5, 3));

        let mode = EditorMode::Normal;
        let status = StatusMessage::Empty;
        let size = (10, 6);

        let frame = build_frame(&editor, &mode, &status, size, None);
        let cursor = frame.cursor_position();
        assert_eq!(cursor.column_index(), 5);
        assert_eq!(cursor.row_index(), 4);
    }

    #[test]
    fn mode_transitions_toggle_command_ui_frame() {
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
        let frame_command = build_frame(&editor, &mode, &status, size, Some(command_ui.frame()));
        assert_eq!(frame_command.mode_label(), "COMMAND");
        assert!(frame_command.command_ui_frame().is_some());

        let _ = handle_request(
            RpcRequest::ModeSet {
                mode: RpcMode::Normal,
            },
            &mut editor,
            &mut mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        let frame_normal = build_frame(&editor, &mode, &status, size, None);
        assert_eq!(frame_normal.mode_label(), "NORMAL");
        assert!(frame_normal.command_ui_frame().is_none());
    }

    #[test]
    fn save_on_existing_path_emits_ack_without_frame() {
        let path = std::env::temp_dir().join("vimrust_rpc_save_exists.txt");
        let _ = fs::write(&path, "existing");

        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut mode = EditorMode::Normal;
        let mut status = StatusMessage::Empty;
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
            assert_eq!(ack.kind(), AckKind::Save);
            assert_eq!(
                ack.message(),
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
            assert_eq!(
                ack.path(),
                FilePath::Provided {
                    path: path.to_string_lossy().to_string(),
                }
            );
        } else {
            panic!("expected ack without frame");
        }
        assert_eq!(
            status,
            StatusMessage::Text {
                text: String::from("saved"),
            }
        );
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "changed");
        let _ = fs::remove_file(&path);
    }
}
