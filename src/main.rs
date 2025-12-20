use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod mode;
mod rpc_client;
mod status_line;
mod terminal;
mod ui;

use mode::EditorMode;
use rpc_client::{ClientEvent, RpcClient};
use terminal::Terminal;
use ui::Ui;
use vimrust_protocol::{
    CommandUiAction,
    DeleteKind,
    MoveDirection,
    PROTOCOL_VERSION,
    RpcMode,
    RpcRequest,
    RpcResponse,
};

fn main() -> io::Result<()> {
    let file_path = env::args().skip(1).next();

    let mut terminal = Terminal::new()?;
    let result = run_rpc_client(&mut terminal, file_path);
    terminal.cleanup();
    result
}

fn run_rpc_client(terminal: &mut Terminal, file_path: Option<String>) -> io::Result<()> {
    let mut client = RpcClient::spawn(file_path)?;
    let mut ui = Ui::new(terminal);

    ui.terminal_update_size()?;
    let size = ui.terminal_size();
    client.send(&RpcRequest::EditorResize {
        cols: size.0,
        rows: size.1,
        suppress_frame: false,
    })?;
    client.send(&RpcRequest::StateGet)?;

    let mut latest_frame = None;
    let mut status_override: Option<String> = None;
    let mut protocol_mismatch: Option<String> = None;

    loop {
        while let Ok(event) = client.receiver.try_recv() {
            match event {
                ClientEvent::Response(resp) => match resp {
                    RpcResponse::Frame(frame) => {
                        latest_frame = Some(frame);
                        status_override = None;
                        if let Some(frame) = &latest_frame {
                            if frame.protocol_version != PROTOCOL_VERSION {
                                protocol_mismatch = Some(format!(
                                    "protocol mismatch: core {} ui {}",
                                    frame.protocol_version, PROTOCOL_VERSION
                                ));
                                let message =
                                    protocol_mismatch.clone().unwrap_or_else(String::new);
                                eprintln!("vimrust: {}", message);
                                return Err(io::Error::new(io::ErrorKind::Other, message));
                            } else {
                                protocol_mismatch = None;
                            }
                        }
                        ui.mark_dirty();
                    }
                    RpcResponse::Ack(ack) => {
                        if protocol_mismatch.is_none() {
                            status_override = ack.message.clone();
                        }
                        ui.set_status_message(status_override.clone());
                    }
                    RpcResponse::Error { message } => {
                        if protocol_mismatch.is_none() {
                            status_override = Some(message);
                        }
                        ui.set_status_message(status_override.clone());
                    }
                },
                ClientEvent::Exited => {
                    ui.set_status_message(Some(String::from("core exited")));
                    ui.set_quit();
                    break;
                }
            }
        }

        if let Some(frame) = &latest_frame {
            let mode = match frame.mode.as_str() {
                "NORMAL" => EditorMode::Normal,
                "EDIT" => EditorMode::Edit,
                "COMMAND" => EditorMode::Command,
                _ => EditorMode::Normal,
            };
            ui.set_mode_external(mode);
            // Prefer explicit status message if set by ack/error.
            let mut frame_to_render = frame.clone();
            if protocol_mismatch.is_some() {
                frame_to_render.status = protocol_mismatch.clone();
            } else if status_override.is_some() {
                frame_to_render.status = status_override.clone();
            }
            ui.render_from_frame(&frame_to_render)?;
        }

        if ui.quit() {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if let Some(ref mut frame) = latest_frame {
                        if ui.status_line().file_message().is_some() {
                            ui.status_line().file_message_clear();
                        }
                        match frame.mode.as_str() {
                            "NORMAL" => match key_event.code {
                                KeyCode::Char('q') => {
                                    client.send(&RpcRequest::EditorQuit)?;
                                }
                                KeyCode::Char('e') => client.send(&RpcRequest::ModeSet {
                                    mode: RpcMode::Edit,
                                })?,
                                KeyCode::Char('s') => {
                                    client.send(&RpcRequest::FileSave)?;
                                }
                                KeyCode::Char(':') => {
                                    client.send(&RpcRequest::ModeSet {
                                        mode: RpcMode::Command,
                                    })?;
                                }
                                KeyCode::Char('h') => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::Left,
                                })?,
                                KeyCode::Char('l') => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::Right,
                                })?,
                                KeyCode::Char('k') => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::Up,
                                })?,
                                KeyCode::Char('j') => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::Down,
                                })?,
                                KeyCode::PageUp => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::PageUp,
                                })?,
                                KeyCode::PageDown => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::PageDown,
                                })?,
                                KeyCode::Home => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::Home,
                                })?,
                                KeyCode::End => client.send(&RpcRequest::CursorMove {
                                    direction: MoveDirection::End,
                                })?,
                                _ => {}
                            },
                            "EDIT" => match key_event.code {
                                KeyCode::Esc => {
                                    client.send(&RpcRequest::ModeSet {
                                        mode: RpcMode::Normal,
                                    })?;
                                }
                                KeyCode::Delete => client.send(&RpcRequest::TextDelete {
                                    kind: DeleteKind::Under,
                                })?,
                                KeyCode::Backspace => client.send(&RpcRequest::TextDelete {
                                    kind: DeleteKind::Backspace,
                                })?,
                                KeyCode::Char(ch) => client.send(&RpcRequest::TextInsert {
                                    text: ch.to_string(),
                                })?,
                                _ => {}
                            },
                            "COMMAND" => match key_event.code {
                                KeyCode::Esc => {
                                    client.send(&RpcRequest::ModeSet {
                                        mode: RpcMode::Normal,
                                    })?;
                                }
                                KeyCode::Enter => {
                                    client.send(&RpcRequest::CommandExecute { line: None })?;
                                }
                                KeyCode::Backspace => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::Backspace,
                                })?,
                                KeyCode::Delete => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::Delete,
                                })?,
                                KeyCode::Left => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::MoveLeft,
                                })?,
                                KeyCode::Right => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::MoveRight,
                                })?,
                                KeyCode::Home => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::MoveHome,
                                })?,
                                KeyCode::End => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::MoveEnd,
                                })?,
                                KeyCode::Up | KeyCode::Down => {
                                    let action = match key_event.code {
                                        KeyCode::Up => CommandUiAction::MoveSelectionUp,
                                        KeyCode::Down => CommandUiAction::MoveSelectionDown,
                                        _ => CommandUiAction::MoveSelectionUp,
                                    };
                                    client.send(&RpcRequest::CommandUi { action })?;
                                }
                                KeyCode::Char(ch) => client.send(&RpcRequest::CommandUi {
                                    action: CommandUiAction::InsertChar { ch },
                                })?,
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                Event::Resize(_, _) => {
                    ui.terminal_update_size()?;
                    let size = ui.terminal_size();
                    client.send(&RpcRequest::EditorResize {
                        cols: size.0,
                        rows: size.1,
                        suppress_frame: false,
                    })?;
                }
                _ => {}
            }
        }
    }

    client.kill();
    Ok(())
}
