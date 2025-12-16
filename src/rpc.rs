use std::io::{self, BufRead, Write};

use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::{editor::Editor, file::File, EditorMode};

/// Minimal stdio-based RPC loop for driving the editor core without the terminal UI.
///
/// Protocol (line-delimited JSON):
/// - {"type":"resize","cols":80,"rows":24}
/// - {"type":"input","key":"h"} (same key semantics as Normal/Edit modes: h/j/k/l, e, s, q, backspace, delete, esc, pageup, pagedown, home, end)
/// - {"type":"render"} (returns a frame)
/// - {"type":"quit"}
/// Any input triggers a frame response so the client can redraw.
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

        let send_frame = true;
        match request {
            RpcRequest::Resize { cols, rows } => {
                size = (cols, rows);
            }
            RpcRequest::Input { key } => {
                let control = handle_key(&key, &mut editor, &mut mode, &mut status_message, size);
                if control == ControlFlow::Quit {
                    break;
                }
            }
            RpcRequest::Render => {
                // fallthrough to send frame
            }
            RpcRequest::Quit => break,
        }

        if send_frame {
            let frame = build_frame(&editor, &mode, &status_message, size);
            if let Err(err) = serde_json::to_writer(&mut stdout, &RpcResponse::Frame(frame)) {
                let _ = write_error(&mut stdout, format!("failed to serialize frame: {}", err));
            } else {
                let _ = stdout.write_all(b"\n");
                let _ = stdout.flush();
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RpcRequest {
    Resize { cols: u16, rows: u16 },
    Input { key: String },
    Render,
    Quit,
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

#[derive(PartialEq)]
enum ControlFlow {
    Continue,
    Quit,
}

fn handle_key(
    key: &str,
    editor: &mut Editor,
    mode: &mut EditorMode,
    status: &mut Option<String>,
    size: (u16, u16),
) -> ControlFlow {
    let usable_rows = size.1.saturating_sub(2);
    match *mode {
        EditorMode::Normal => match key {
            "q" => return ControlFlow::Quit,
            "e" => {
                *mode = EditorMode::Edit;
                editor.snap_cursor_to_tab_start();
            }
            "s" => match editor.save(&mut None) {
                Ok(msg) => *status = Some(msg),
                Err(err) => *status = Some(format!("Error saving: {}", err)),
            },
            ":" => {
                *mode = EditorMode::Command;
            }
            "h" | "left" => editor.cursor_move(KeyCode::Char('h'), usable_rows),
            "j" | "down" => editor.cursor_move(KeyCode::Char('j'), usable_rows),
            "k" | "up" => editor.cursor_move(KeyCode::Char('k'), usable_rows),
            "l" | "right" => editor.cursor_move(KeyCode::Char('l'), usable_rows),
            "pageup" => editor.cursor_move(KeyCode::PageUp, usable_rows),
            "pagedown" => editor.cursor_move(KeyCode::PageDown, usable_rows),
            "home" => editor.cursor_move(KeyCode::Home, usable_rows),
            "end" => editor.cursor_move(KeyCode::End, usable_rows),
            other if other.len() == 1 => {
                // Unknown single-char in normal mode: ignore.
                let _ = other;
            }
            _ => {}
        },
        EditorMode::Edit => match key {
            "esc" => {
                *mode = EditorMode::Normal;
            }
            "backspace" => editor.delete_backspace(),
            "delete" => editor.delete_under_cursor(),
            "pageup" => editor.cursor_move(KeyCode::PageUp, usable_rows),
            "pagedown" => editor.cursor_move(KeyCode::PageDown, usable_rows),
            "home" => editor.cursor_move(KeyCode::Home, usable_rows),
            "end" => editor.cursor_move(KeyCode::End, usable_rows),
            other if other.len() == 1 => {
                if let Some(ch) = other.chars().next() {
                    editor.insert_char(ch);
                }
            }
            _ => {}
        },
        EditorMode::Command => {
            // Command mode not yet implemented in RPC; ESC returns to normal.
            if key == "esc" {
                *mode = EditorMode::Normal;
            }
        }
    }

    ControlFlow::Continue
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
