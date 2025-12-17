use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod command_line;
mod command_list;
mod core;
mod editor;
mod file;
mod rpc;
mod status_line;
mod terminal;
mod ui;

use command_line::CommandLine;
use command_list::CommandList;
use core::CoreState;
use rpc::{DeleteKind, MoveDir, RequestOutcome, RpcMode, RpcRequest};
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

fn run(terminal: &mut Terminal, file_path: Option<String>) -> io::Result<()> {
    let mut core = CoreState::new(file_path.clone());
    let mut command_list = CommandList::new();
    let mut command_line = CommandLine::new();

    let result: io::Result<()> = {
        let mut ui = Ui::new(terminal, &mut command_line, &mut command_list);
        core.read_file()?;
        ui.terminal_update_size()?;
        core.set_size(ui.terminal_size());

        loop {
            let command_ui = if matches!(core.mode(), EditorMode::Command) {
                Some(ui.command_ui_snapshot())
            } else {
                None
            };
            let frame = core.frame(command_ui);
            ui.set_mode_external(core.mode());
            ui.render_from_frame(&frame)?;

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if ui.status_line().message().is_some() {
                            ui.status_line().message_clear();
                        }
                        match core.mode() {
                            EditorMode::Normal => match key_event.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('e') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Edit,
                                        },
                                    )?;
                                }
                                KeyCode::Char('s') => {
                                    handle_core_request(&mut core, &mut ui, RpcRequest::Save)?;
                                }
                                KeyCode::Char(':') => {
                                    ui.mode_command_enter()?;
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Command,
                                        },
                                    )?;
                                }
                                KeyCode::Char('h') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Left,
                                        },
                                    )?;
                                }
                                KeyCode::Char('l') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Right,
                                        },
                                    )?;
                                }
                                KeyCode::Char('k') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Up,
                                        },
                                    )?;
                                }
                                KeyCode::Char('j') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Down,
                                        },
                                    )?;
                                }
                                KeyCode::PageUp => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::PageUp,
                                        },
                                    )?;
                                }
                                KeyCode::PageDown => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::PageDown,
                                        },
                                    )?;
                                }
                                KeyCode::Home => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::Home,
                                        },
                                    )?;
                                }
                                KeyCode::End => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::MoveCursor {
                                            direction: MoveDir::End,
                                        },
                                    )?;
                                }
                                _ => {}
                            },
                            EditorMode::Edit => match key_event.code {
                                KeyCode::Esc => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Normal,
                                        },
                                    )?;
                                }
                                KeyCode::Delete => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::Delete {
                                            kind: DeleteKind::Under,
                                        },
                                    )?;
                                }
                                KeyCode::Backspace => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::Delete {
                                            kind: DeleteKind::Backspace,
                                        },
                                    )?;
                                }
                                KeyCode::Char(ch) => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
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
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::SetMode {
                                            mode: RpcMode::Normal,
                                        },
                                    )?;
                                }
                                KeyCode::Enter => {
                                    let requests = ui.command_enter()?;
                                    for req in requests {
                                        handle_core_request(&mut core, &mut ui, req)?;
                                    }
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
                        let size = ui.terminal_size();
                        core.set_size(size);
                        handle_core_request(
                            &mut core,
                            &mut ui,
                            RpcRequest::Resize {
                                cols: size.0,
                                rows: size.1,
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

fn handle_core_request(core: &mut CoreState, ui: &mut Ui, request: RpcRequest) -> io::Result<()> {
    let outcome = core.handle(request);
    match outcome {
        RequestOutcome::Frame => {
            ui.set_status_message(core.status().clone());
        }
        RequestOutcome::Ack(ack) => {
            ui.set_status_message(ack.message.clone());
        }
        RequestOutcome::FrameAndAck(ack) => {
            ui.set_status_message(ack.message.clone());
        }
        RequestOutcome::Skip => {}
        RequestOutcome::Quit => {
            ui.set_status_message(core.status().clone());
            ui.set_quit();
        }
        RequestOutcome::Error(message) => {
            ui.set_status_message(Some(message));
        }
    }
    ui.mark_dirty();
    ui.set_mode_external(core.mode());
    Ok(())
}
