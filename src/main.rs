use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod command_line;
mod command_list;
mod command_ui_state;
mod core;
mod editor;
mod file;
mod rpc;
mod status_line;
mod terminal;
mod ui;

use core::CoreState;
use command_ui_state::CommandUiAction;
use rpc::{
    DeleteKind,
    MoveDir,
    RequestOutcome,
    RpcMode,
    RpcRequest,
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

fn run(terminal: &mut Terminal, file_path: Option<String>) -> io::Result<()> {
    let mut core = CoreState::new(file_path.clone());

    let result: io::Result<()> = {
        let mut ui = Ui::new(terminal);
        core.read_file()?;
        ui.terminal_update_size()?;
        core.set_size(ui.terminal_size());

        loop {
            let frame = core.frame();
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
                                        RpcRequest::ModeSet {
                                            mode: RpcMode::Edit,
                                        },
                                    )?;
                                }
                                KeyCode::Char('s') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::FileSave,
                                    )?;
                                }
                                KeyCode::Char(':') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::ModeSet {
                                            mode: RpcMode::Command,
                                        },
                                    )?;
                                }
                                KeyCode::Char('h') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::Left,
                                        },
                                    )?;
                                }
                                KeyCode::Char('l') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::Right,
                                        },
                                    )?;
                                }
                                KeyCode::Char('k') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::Up,
                                        },
                                    )?;
                                }
                                KeyCode::Char('j') => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::Down,
                                        },
                                    )?;
                                }
                                KeyCode::PageUp => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::PageUp,
                                        },
                                    )?;
                                }
                                KeyCode::PageDown => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::PageDown,
                                        },
                                    )?;
                                }
                                KeyCode::Home => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
                                            direction: MoveDir::Home,
                                        },
                                    )?;
                                }
                                KeyCode::End => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CursorMove {
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
                                        RpcRequest::ModeSet {
                                            mode: RpcMode::Normal,
                                        },
                                    )?;
                                }
                                KeyCode::Delete => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::TextDelete {
                                            kind: DeleteKind::Under,
                                        },
                                    )?;
                                }
                                KeyCode::Backspace => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::TextDelete {
                                            kind: DeleteKind::Backspace,
                                        },
                                    )?;
                                }
                                KeyCode::Char(ch) => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::TextInsert {
                                            text: ch.to_string(),
                                        },
                                    )?;
                                }
                                _ => {}
                            },
                            EditorMode::Command => match key_event.code {
                                KeyCode::Esc => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::ModeSet {
                                            mode: RpcMode::Normal,
                                        },
                                    )?;
                                }
                                KeyCode::Enter => {
                                    let was_focused_on_list = frame
                                        .command_ui
                                        .as_ref()
                                        .map(|c| c.focus_on_list)
                                        .unwrap_or(false);
                                    if was_focused_on_list {
                                        handle_core_request(
                                            &mut core,
                                            &mut ui,
                                            RpcRequest::CommandUi {
                                                action: CommandUiAction::SelectFromList,
                                            },
                                        )?;
                                    } else if let Some(cmd_ui) = &frame.command_ui {
                                        handle_core_request(
                                            &mut core,
                                            &mut ui,
                                            RpcRequest::CommandExecute {
                                                line: cmd_ui.line.clone(),
                                            },
                                        )?;
                                    }
                                }
                                KeyCode::Backspace => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::Backspace,
                                        },
                                    )?;
                                }
                                KeyCode::Delete => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::Delete,
                                        },
                                    )?;
                                }
                                KeyCode::Left => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::MoveLeft,
                                        },
                                    )?;
                                }
                                KeyCode::Right => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::MoveRight,
                                        },
                                    )?;
                                }
                                KeyCode::Home => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::MoveHome,
                                        },
                                    )?;
                                }
                                KeyCode::End => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::MoveEnd,
                                        },
                                    )?;
                                }
                                KeyCode::Up | KeyCode::Down => {
                                    let action = match key_event.code {
                                        KeyCode::Up => CommandUiAction::MoveSelectionUp,
                                        KeyCode::Down => CommandUiAction::MoveSelectionDown,
                                        _ => CommandUiAction::MoveSelectionUp,
                                    };
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi { action },
                                    )?;
                                }
                                KeyCode::Char(ch) => {
                                    handle_core_request(
                                        &mut core,
                                        &mut ui,
                                        RpcRequest::CommandUi {
                                            action: CommandUiAction::InsertChar { ch },
                                        },
                                    )?;
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
                            RpcRequest::EditorResize {
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
