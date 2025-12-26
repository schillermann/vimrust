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
use rpc_client::{ClientEvent, ClientFilePath, ClientPoll, RpcClient};
use terminal::Terminal;
use ui::Ui;
use vimrust_protocol::{
    CommandUiAction, DeleteKind, MoveDirection, ProtocolVersion, RpcMode, RpcRequest, RpcResponse,
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
    let mut protocol_gate = ProtocolGate::new(ProtocolVersion::current());
    let keymap = ModeKeymap::new();

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
                                protocol_gate.observe(frame.version());
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
            let mode = FrameMode { label: frame.mode_label() }.editor_mode();
            let mut frame_to_render = frame.clone();
            ui.mode_apply(mode);
            // Prefer explicit status message if set by ack/error.
            let status = protocol_gate
                .status()
                .or(status_override.clone())
                .or(frame.status_message());
            frame_to_render.status_update(status);
            ui.render_from_frame(&frame_to_render)?;
        }

        if ui.quit() {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if let Some(ref mut frame) = latest_frame {
                        let mode = FrameMode { label: frame.mode_label() }.editor_mode();
                        let action = keymap.action_for(mode, key_event.code);
                        ui.status_clear();
                        action.apply(&mut client)?;
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

struct FrameMode<'a> {
    label: &'a str,
}

impl<'a> FrameMode<'a> {
    fn editor_mode(&self) -> EditorMode {
        match self.label {
            "NORMAL" => EditorMode::Normal,
            "EDIT" => EditorMode::Edit,
            "COMMAND" => EditorMode::Command,
            _ => EditorMode::Normal,
        }
    }
}

struct ModeKeymap {
    normal: NormalModeInput,
    edit: EditModeInput,
    command: CommandModeInput,
}

impl ModeKeymap {
    fn new() -> Self {
        Self {
            normal: NormalModeInput,
            edit: EditModeInput,
            command: CommandModeInput,
        }
    }

    fn action_for(&self, mode: EditorMode, code: KeyCode) -> ClientAction {
        match mode {
            EditorMode::Normal => self.normal.action(code),
            EditorMode::Edit => self.edit.action(code),
            EditorMode::Command => self.command.action(code),
        }
    }
}

struct NormalModeInput;

impl NormalModeInput {
    fn action(&self, code: KeyCode) -> ClientAction {
        match code {
            KeyCode::Char('q') => ClientAction::Send(RpcRequest::EditorQuit),
            KeyCode::Char('e') => ClientAction::Send(RpcRequest::ModeSet { mode: RpcMode::Edit }),
            KeyCode::Char('s') => ClientAction::Send(RpcRequest::FileSave),
            KeyCode::Char(':') => {
                ClientAction::Send(RpcRequest::ModeSet { mode: RpcMode::Command })
            }
            KeyCode::Char('h') => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::Left,
            }),
            KeyCode::Char('l') => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::Right,
            }),
            KeyCode::Char('k') => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::Up,
            }),
            KeyCode::Char('j') => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::Down,
            }),
            KeyCode::PageUp => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::PageUp,
            }),
            KeyCode::PageDown => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::PageDown,
            }),
            KeyCode::Home => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::Home,
            }),
            KeyCode::End => ClientAction::Send(RpcRequest::CursorMove {
                direction: MoveDirection::End,
            }),
            _ => ClientAction::Skip,
        }
    }
}

struct EditModeInput;

impl EditModeInput {
    fn action(&self, code: KeyCode) -> ClientAction {
        match code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet { mode: RpcMode::Normal }),
            KeyCode::Delete => ClientAction::Send(RpcRequest::TextDelete {
                kind: DeleteKind::Under,
            }),
            KeyCode::Backspace => ClientAction::Send(RpcRequest::TextDelete {
                kind: DeleteKind::Backspace,
            }),
            KeyCode::Char(ch) => ClientAction::Send(RpcRequest::TextInsert {
                text: ch.to_string(),
            }),
            _ => ClientAction::Skip,
        }
    }
}

struct CommandModeInput;

impl CommandModeInput {
    fn action(&self, code: KeyCode) -> ClientAction {
        match code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet { mode: RpcMode::Normal }),
            KeyCode::Enter => ClientAction::Send(RpcRequest::CommandExecute { line: None }),
            KeyCode::Backspace => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::Backspace,
            }),
            KeyCode::Delete => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::Delete,
            }),
            KeyCode::Left => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveLeft,
            }),
            KeyCode::Right => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveRight,
            }),
            KeyCode::Home => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveHome,
            }),
            KeyCode::End => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveEnd,
            }),
            KeyCode::Up => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveSelectionUp,
            }),
            KeyCode::Down => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::MoveSelectionDown,
            }),
            KeyCode::Char(ch) => ClientAction::Send(RpcRequest::CommandUi {
                action: CommandUiAction::InsertChar { ch },
            }),
            _ => ClientAction::Skip,
        }
    }
}

enum ClientAction {
    Send(RpcRequest),
    Skip,
}

impl ClientAction {
    fn apply(&self, client: &mut RpcClient) -> io::Result<()> {
        match self {
            ClientAction::Send(request) => client.send(request),
            ClientAction::Skip => Ok(()),
        }
    }
}
