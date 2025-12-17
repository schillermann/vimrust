use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod command_line;
mod command_list;
mod editor;
mod file;
mod rpc;
mod status_line;
mod terminal;
mod ui;

use command_line::CommandLine;
use command_list::CommandList;
use editor::Editor;
use file::File;
use rpc::{
    DeleteKind, MoveDir, RequestOutcome, RpcMode, RpcRequest, build_frame, handle_request,
};
use terminal::Terminal;
use ui::Ui;

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Normal,
    Edit,
    Command,
}

impl EditorMode {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Edit => "EDIT",
            EditorMode::Command => "COMMAND",
        }
    }
}

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let first_arg = args.next();
    let rpc_mode = first_arg.as_deref() == Some("--rpc");
    let file_path = if rpc_mode { args.next() } else { first_arg };

    if rpc_mode {
        return rpc::serve_stdio(file_path);
    }

    let mut terminal = Terminal::new()?;
    let result = run(&mut terminal, file_path);
    terminal.cleanup();
    result
}

fn run(terminal: &mut Terminal, mut file_path: Option<String>) -> io::Result<()> {
    let file = File::new(file_path.clone());
    let mut editor = Editor::new(file);
    let mut command_list = CommandList::new();
    let mut command_line = CommandLine::new();
    let mut status: Option<String> = None;
    let mut mode = EditorMode::Normal;

    let result: io::Result<()> = {
        let mut ui = Ui::new(terminal, &mut editor, &mut command_line, &mut command_list);
        ui.set_mode(EditorMode::Normal)?;
        ui.editor().file_read()?;
        ui.terminal_update_size()?;
        let mut size = ui.terminal_size();

        loop {
            let command_ui = if matches!(mode, EditorMode::Command) {
                Some(ui.command_ui_snapshot())
            } else {
                None
            };
            let frame = build_frame(ui.editor_ref(), &mode, &status, size, command_ui);
            ui.render_from_frame(&frame)?;

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if ui.status_line().message().is_some() {
                            ui.status_line().message_clear();
                        }
                        match *ui.mode() {
                            EditorMode::Normal => match key_event.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('e') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Edit,
                                        },
                                    )?;
                                }
                                KeyCode::Char('s') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::Save,
                                    )?;
                                }
                                KeyCode::Char(':') => {
                                    ui.mode_command_enter()?;
                                    mode = EditorMode::Command;
                                }
                                KeyCode::Char('h') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Left,
                                        },
                                    )?;
                                }
                                KeyCode::Char('l') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Right,
                                        },
                                    )?;
                                }
                                KeyCode::Char('k') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Up,
                                        },
                                    )?;
                                }
                                KeyCode::Char('j') => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Down,
                                        },
                                    )?;
                                }
                                KeyCode::PageUp => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::PageUp,
                                        },
                                    )?;
                                }
                                KeyCode::PageDown => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::PageDown,
                                        },
                                    )?;
                                }
                                KeyCode::Home => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Home,
                                        },
                                    )?;
                                }
                                KeyCode::End => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::End,
                                        },
                                    )?;
                                }
                                _ => {}
                            },
                            EditorMode::Edit => match key_event.code {
                                KeyCode::Esc => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Normal,
                                        },
                                    )?;
                                }
                                KeyCode::Delete => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::Delete {
                                            kind: DeleteKind::Under,
                                        },
                                    )?;
                                }
                                KeyCode::Backspace => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::Delete {
                                            kind: DeleteKind::Backspace,
                                        },
                                    )?;
                                }
                                KeyCode::Char(ch) => {
                                    process_request(
                                        &mut ui,
                                        &mut file_path,
                                        &mut status,
                                        &mut size,
                                        &mut mode,
                                        RpcRequest::Insert {
                                            text: ch.to_string(),
                                        },
                                    )?;
                                }
                                _ => {}
                            },
                            EditorMode::Command => match key_event.code {
                                KeyCode::Esc => {
                                    ui.mode_command_exit()?;
                                    mode = EditorMode::Normal;
                                }
                                KeyCode::Enter => {
                                    ui.command_enter(&mut file_path)?;
                                    mode = *ui.mode();
                                }
                                KeyCode::Backspace => {
                                    ui.command_line_backspace();
                                }
                                KeyCode::Delete => {
                                    ui.command_line_delete();
                                }
                                KeyCode::Left => {
                                    ui.command_line_move_left();
                                }
                                KeyCode::Right => {
                                    ui.command_line_move_right();
                                }
                                KeyCode::Home => {
                                    ui.command_line_move_home();
                                }
                                KeyCode::End => {
                                    ui.command_line_move_end();
                                }
                                KeyCode::Up | KeyCode::Down => {
                                    ui.command_list_move_selection(key_event.code);
                                }
                                KeyCode::Char(ch) => {
                                    ui.command_line_insert_char(ch);
                                }
                                _ => {}
                            },
                        }
                    }
                    Event::Resize(_, _) => {
                        ui.terminal_update_size()?;
                        size = ui.terminal_size();
                        let cols = size.0;
                        let rows = size.1;
                        process_request(
                            &mut ui,
                            &mut file_path,
                            &mut status,
                            &mut size,
                            &mut mode,
                            RpcRequest::Resize {
                                cols,
                                rows,
                                suppress_frame: false,
                            },
                        )?;
                    }
                    _ => {}
                }

                if ui.quit() {
                    break;
                }
            } else {
                // periodic tasks / redraw can go here
            }
        }
        Ok(())
    };

    result
}

fn process_request(
    ui: &mut Ui,
    file_path: &mut Option<String>,
    status: &mut Option<String>,
    size: &mut (u16, u16),
    mode: &mut EditorMode,
    request: RpcRequest,
) -> io::Result<()> {
    match handle_request(request, ui.editor(), mode, status, size) {
        RequestOutcome::Frame => {
            ui.set_status_message(status.clone());
        }
        RequestOutcome::Ack(ack) => {
            ui.set_status_message(ack.message.clone());
            if let Some(path) = ack.file_path {
                *file_path = Some(path);
            }
        }
        RequestOutcome::FrameAndAck(ack) => {
            ui.set_status_message(ack.message.clone());
            if let Some(path) = ack.file_path {
                *file_path = Some(path);
            }
        }
        RequestOutcome::Skip => {}
        RequestOutcome::Quit => {
            ui.set_status_message(status.clone());
            ui.set_quit();
        }
        RequestOutcome::Error(message) => {
            ui.set_status_message(Some(message));
        }
    }

    *file_path = ui.editor().file.path().cloned();
    ui.mark_dirty();
    ui.set_mode_external(*mode);
    Ok(())
}
