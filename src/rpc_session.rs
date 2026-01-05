use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    mode::EditorMode,
    protocol_guard::ProtocolGate,
    rpc_client::{ClientEvent, ClientPoll, RpcClient},
    ui::Ui,
};
use vimrust_protocol::{
    CommandUiAccess, DeleteKind, Frame, MoveDirection, PromptUiAction, RequestEditorMode,
    RpcRequest, RpcResponse, StatusMessage,
};

// `'a` is the lifetime of the borrowed terminal inside Ui.
// It guarantees the borrow lasts as long as the session does.
pub struct RpcSession<'a> {
    client: RpcClient,
    ui: Ui<'a>,
    protocol_gate: ProtocolGate,
    keymap: ModeKeymap,
    latest_frame: Option<Frame>,
    status_override: StatusMessage,
}

impl<'a> RpcSession<'a> {
    pub fn new(
        client: RpcClient,
        ui: Ui<'a>,
        protocol_gate: ProtocolGate,
        keymap: ModeKeymap,
    ) -> Self {
        Self {
            client,
            ui,
            protocol_gate,
            keymap,
            latest_frame: None,
            status_override: StatusMessage::Empty,
        }
    }

    pub fn open(&mut self) -> io::Result<()> {
        self.handshake()?;
        loop {
            self.receive()?;
            self.render()?;
            if matches!(self.ui.quit_signal(), crate::ui::UiQuitSignal::Requested) {
                break;
            }
            self.listen()?;
        }
        self.client.kill();
        Ok(())
    }

    fn handshake(&mut self) -> io::Result<()> {
        self.ui.terminal_update_size()?;
        let request = self.ui.resize_request(false);
        self.client.send(&request)?;
        self.client.send(&RpcRequest::StateGet)?;
        Ok(())
    }

    fn receive(&mut self) -> io::Result<()> {
        loop {
            let poll = self.client.poll_event()?;
            match poll {
                ClientPoll::Event(event) => self.accept_event(event)?,
                ClientPoll::Empty => break,
            }
        }
        Ok(())
    }

    fn accept_event(&mut self, event: ClientEvent) -> io::Result<()> {
        match event {
            ClientEvent::Response(resp) => self.accept_event_response(resp),
            ClientEvent::Exited => {
                self.ui.status_update(StatusMessage::Text {
                    text: String::from("core exited"),
                });
                self.ui.quit_request();
                Ok(())
            }
        }
    }

    fn accept_event_response(&mut self, response: RpcResponse) -> io::Result<()> {
        match response {
            RpcResponse::Frame(frame) => self.accept_frame(frame),
            RpcResponse::Ack(ack) => self.accept_ack(ack),
            RpcResponse::Error { message } => self.accept_error(message),
        }
    }

    fn accept_frame(&mut self, frame: Frame) -> io::Result<()> {
        self.latest_frame = Some(frame);
        self.status_override = StatusMessage::Empty;
        if let Some(frame) = &self.latest_frame {
            self.protocol_gate.observe(frame.protocol());
            self.protocol_gate.report();
            self.protocol_gate.result()?;
        }
        self.ui.mark_dirty();
        Ok(())
    }

    fn accept_ack(&mut self, ack: vimrust_protocol::Ack) -> io::Result<()> {
        self.status_override = self.protocol_gate.status().or(ack.message());
        self.ui.status_update(self.status_override.clone());
        Ok(())
    }

    fn accept_error(&mut self, message: String) -> io::Result<()> {
        self.status_override = self
            .protocol_gate
            .status()
            .or(StatusMessage::Text { text: message });
        self.ui.status_update(self.status_override.clone());
        Ok(())
    }

    fn render(&mut self) -> io::Result<()> {
        if let Some(frame) = &self.latest_frame {
            let mode = UiFrameEditorMode::new(frame.mode()).editor_mode();
            let mut frame_to_render = frame.clone();
            self.ui.mode_apply(mode);
            // Prefer explicit status message if set by ack/error.
            let status = self
                .protocol_gate
                .status()
                .or(self.status_override.clone())
                .or(frame.status());
            frame_to_render.status_update(status);
            self.ui.render_from_frame(&frame_to_render)?;
        }
        Ok(())
    }

    fn listen(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if let Some(ref mut frame) = self.latest_frame {
                        let mode = UiFrameEditorMode::new(frame.mode()).editor_mode();
                        let focus = PromptFocus::new(frame);
                        let action = self.keymap.action_for(mode, key_event, focus);
                        self.ui.status_clear();
                        action.apply(&mut self.client)?;
                    }
                }
                Event::Resize(_, _) => {
                    self.ui.terminal_update_size()?;
                    let request = self.ui.resize_request(false);
                    self.client.send(&request)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

struct UiFrameEditorMode {
    mode: vimrust_protocol::FrameEditorMode,
}

impl UiFrameEditorMode {
    fn new(mode: vimrust_protocol::FrameEditorMode) -> Self {
        Self { mode }
    }

    fn editor_mode(&self) -> EditorMode {
        match self.mode {
            vimrust_protocol::FrameEditorMode::Normal => EditorMode::Normal,
            vimrust_protocol::FrameEditorMode::Edit => EditorMode::Edit,
            vimrust_protocol::FrameEditorMode::Visual => EditorMode::Visual,
            vimrust_protocol::FrameEditorMode::PromptCommand => EditorMode::PromptCommand,
            vimrust_protocol::FrameEditorMode::PromptKeymap => EditorMode::PromptKeymap,
        }
    }
}

pub struct ModeKeymap {
    normal: NormalModeInput,
    edit: EditModeInput,
    visual: VisualModeInput,
    prompt_command: PromptPromptInput,
    prompt_keymap: PromptKeymapInput,
}

impl ModeKeymap {
    pub fn new() -> Self {
        Self {
            normal: NormalModeInput,
            edit: EditModeInput,
            visual: VisualModeInput,
            prompt_command: PromptPromptInput,
            prompt_keymap: PromptKeymapInput,
        }
    }

    fn action_for(&self, mode: EditorMode, event: KeyEvent, focus: PromptFocus) -> ClientAction {
        match mode {
            EditorMode::Normal => self.normal.action(event),
            EditorMode::Edit => self.edit.action(event),
            EditorMode::Visual => self.visual.action(event),
            EditorMode::PromptCommand => self.prompt_command.action(event, focus),
            EditorMode::PromptKeymap => self.prompt_keymap.action(event),
        }
    }
}

struct NormalModeInput;

impl NormalModeInput {
    fn action(&self, event: KeyEvent) -> ClientAction {
        match event.code {
            KeyCode::Char('q') => ClientAction::Send(RpcRequest::EditorQuit),
            KeyCode::Char('e') => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Edit,
            }),
            KeyCode::Char('v') => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Visual,
            }),
            KeyCode::Char('s') => ClientAction::Send(RpcRequest::FileSave),
            KeyCode::Char(':') => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::PromptCommand,
            }),
            KeyCode::Char(';') => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::PromptKeymap,
            }),
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
    fn action(&self, event: KeyEvent) -> ClientAction {
        match event.code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Normal,
            }),
            KeyCode::Delete => ClientAction::Send(RpcRequest::TextDelete {
                kind: DeleteKind::Under,
            }),
            KeyCode::Backspace => ClientAction::Send(RpcRequest::TextDelete {
                kind: DeleteKind::Backspace,
            }),
            KeyCode::Enter => ClientAction::Send(RpcRequest::LineBreak),
            KeyCode::Char(ch) => ClientAction::Send(RpcRequest::TextInsert {
                text: ch.to_string(),
            }),
            _ => ClientAction::Skip,
        }
    }
}

struct VisualModeInput;

impl VisualModeInput {
    fn action(&self, event: KeyEvent) -> ClientAction {
        match event.code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Normal,
            }),
            KeyCode::Char(':') => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::PromptCommand,
            }),
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

struct PromptPromptInput;

impl PromptPromptInput {
    fn action(&self, event: KeyEvent, focus: PromptFocus) -> ClientAction {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Up => {
                    return ClientAction::Send(RpcRequest::CommandUi {
                        action: PromptUiAction::FocusPrompt,
                    });
                }
                KeyCode::Down => {
                    return ClientAction::Send(RpcRequest::CommandUi {
                        action: PromptUiAction::MoveSelectionDown,
                    });
                }
                _ => {}
            }
        }
        match event.code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Normal,
            }),
            KeyCode::Enter => match focus {
                PromptFocus::List => ClientAction::Send(RpcRequest::CommandUi {
                    action: PromptUiAction::SelectFromList,
                }),
                PromptFocus::Input => ClientAction::Send(RpcRequest::CommandExecute { line: None }),
            },
            KeyCode::Backspace => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::Backspace,
            }),
            KeyCode::Delete => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::Delete,
            }),
            KeyCode::Left => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveLeft,
            }),
            KeyCode::Right => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveRight,
            }),
            KeyCode::Home => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveHome,
            }),
            KeyCode::End => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveEnd,
            }),
            KeyCode::Tab => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::Complete,
            }),
            KeyCode::Up => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::HistoryPrevious,
            }),
            KeyCode::Down => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::HistoryNext,
            }),
            KeyCode::Char(ch) => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::InsertChar { ch },
            }),
            _ => ClientAction::Skip,
        }
    }
}

struct PromptKeymapInput;

impl PromptKeymapInput {
    fn action(&self, event: KeyEvent) -> ClientAction {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Up => {
                    return ClientAction::Send(RpcRequest::CommandUi {
                        action: PromptUiAction::FocusPrompt,
                    });
                }
                KeyCode::Down => {
                    return ClientAction::Send(RpcRequest::CommandUi {
                        action: PromptUiAction::MoveSelectionDown,
                    });
                }
                _ => {}
            }
        }
        match event.code {
            KeyCode::Esc => ClientAction::Send(RpcRequest::ModeSet {
                mode: RequestEditorMode::Normal,
            }),
            KeyCode::Backspace => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::Backspace,
            }),
            KeyCode::Delete => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::Delete,
            }),
            KeyCode::Left => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveLeft,
            }),
            KeyCode::Right => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveRight,
            }),
            KeyCode::Home => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveHome,
            }),
            KeyCode::End => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveEnd,
            }),
            KeyCode::Up => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveSelectionUp,
            }),
            KeyCode::Down => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::MoveSelectionDown,
            }),
            KeyCode::Char(ch) => ClientAction::Send(RpcRequest::CommandUi {
                action: PromptUiAction::InsertChar { ch },
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

enum PromptFocus {
    Input,
    List,
}

impl PromptFocus {
    fn new(frame: &Frame) -> Self {
        match frame.command_ui() {
            CommandUiAccess::Available(cmd_ui) => match cmd_ui.line_focus() {
                true => PromptFocus::Input,
                false => PromptFocus::List,
            },
            CommandUiAccess::Missing => PromptFocus::Input,
        }
    }
}
