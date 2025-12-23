use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod mode;
mod protocol_guard;
mod rpc_client;
mod status_line;
mod terminal;
mod ui;

use mode::EditorMode;
use protocol_guard::ProtocolGate;
use rpc_client::{ClientEvent, ClientFilePath, ClientPoll};
use terminal::Terminal;
use ui::Ui;
use vimrust_protocol::{
    CommandUiAction, DeleteKind, MoveDirection, PROTOCOL_VERSION, RpcMode, RpcRequest, RpcResponse,
    StatusMessage,
};

fn main() -> io::Result<()> {
    let file_path = ArgFilePath { args: env::args() }.read();

    let mut terminal = Terminal::new()?;
    let result = run_rpc_client(&mut terminal, file_path);
    terminal.cleanup();
    result
}

fn run_rpc_client(terminal: &mut Terminal, file_path: ClientFilePath) -> io::Result<()> {
    let launcher = file_path.launcher();
    let mut client = launcher.launch()?;
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
    let mut status_override = StatusMessage::Empty;
    let mut protocol_gate = ProtocolGate::new(PROTOCOL_VERSION);

    loop {
        loop {
            let poll = client.poll_event()?;
            match poll {
                ClientPoll::Event(event) => match event {
                    ClientEvent::Response(resp) => match resp {
                        RpcResponse::Frame(frame) => {
                            latest_frame = Some(frame);
                            status_override = StatusMessage::Empty;
                            if let Some(frame) = &latest_frame {
                                protocol_gate.observe(frame.protocol_version());
                                protocol_gate.report();
                                protocol_gate.result()?;
                            }
                            ui.mark_dirty();
                        }
                        RpcResponse::Ack(ack) => {
                            status_override = protocol_gate.status().or(ack.message());
                            ui.status_update(status_override.clone());
                        }
                        RpcResponse::Error { message } => {
                            status_override = protocol_gate
                                .status()
                                .or(StatusMessage::Text { text: message });
                            ui.status_update(status_override.clone());
                        }
                    },
                    ClientEvent::Exited => {
                        ui.status_update(StatusMessage::Text {
                            text: String::from("core exited"),
                        });
                        ui.quit_request();
                        break;
                    }
                },
                ClientPoll::Empty => break,
            }
        }

        if let Some(frame) = &latest_frame {
            let mode = match frame.mode() {
                "NORMAL" => EditorMode::Normal,
                "EDIT" => EditorMode::Edit,
                "COMMAND" => EditorMode::Command,
                _ => EditorMode::Normal,
            };
            let mut frame_to_render = frame.clone();
            ui.mode_apply(mode);
            // Prefer explicit status message if set by ack/error.
            let status = protocol_gate
                .status()
                .or(status_override.clone())
                .or(frame.status());
            frame_to_render.status_store(status);
            ui.render_from_frame(&frame_to_render)?;
        }

        if ui.quit() {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if let Some(ref mut frame) = latest_frame {
                        ui.status_clear();
                        match frame.mode() {
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

struct ArgFilePath {
    args: env::Args,
}

impl ArgFilePath {
    fn read(mut self) -> ClientFilePath {
        let _ = self.args.next();
        match self.args.next() {
            Some(path) => ClientFilePath::Provided(path),
            None => ClientFilePath::Missing,
        }
    }
}
