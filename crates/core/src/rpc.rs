use std::io::{self, BufRead, Write};

use crate::{
    EditorMode, EditorModeState, FrameSignal, editor::Editor, file::File,
    prompt_ui_state::PromptUiState,
};
use vimrust_protocol::{
    Ack, AckKind, PromptInputSelection, Cursor, DeleteKind, FilePath, Frame, FrameEditorMode,
    MoveDirection, PromptUiAction, PromptUiFrame, ProtocolVersion, RequestEditorMode, RpcRequest,
    RpcResponse, StatusMessage, StatusPosition,
};

/// Line-delimited JSON RPC session for driving the editor core without the terminal UI.
///
/// Requests (`"type"` field):
/// - editor_resize: {"type":"editor_resize","cols":80,"rows":24}
/// - file_open: {"type":"file_open","path":"/tmp/file.txt"}
/// - file_save: {"type":"file_save"}
/// - file_save_as: {"type":"file_save_as","path":"/tmp/new.txt"}
/// - text_insert: {"type":"text_insert","text":"hello"}
/// - text_delete: {"type":"text_delete","kind":"backspace"|"under"}
/// - line_break: {"type":"line_break"}
/// - cursor_move: {"type":"cursor_move","direction":"left"|"right"|"up"|"down"|"page_up"|"page_down"|"home"|"end"}
/// - command_ui: {"type":"command_ui","action":"insert_char","ch":"a"} (for command-line editing/navigation)
/// - mode_set: {"type":"mode_set","mode":"normal"|"edit"|"visual"|"prompt_command"|"prompt_keymap"}
/// - state_get: {"type":"state_get"}
/// - editor_quit: {"type":"editor_quit"}
/// - command_execute: {"type":"command_execute","line":":s"} (execute entered command text; if
///   omitted, the current command line stored in the core is used)
///
/// Responses:
/// - frame: {"type":"frame", ...} for state snapshots (emitted after state changes and on get_state)
/// - ack: {"type":"ack","kind":"save"|"save_as"|"open","message":{"kind":"text","text":"..."},"file_path":{"kind":"provided","path":"/tmp/foo.txt"}} for success confirmation
/// - error: {"type":"error","message":"..."} on failure
pub struct StdioSession {
    file_path: FilePath,
}

impl StdioSession {
    pub fn new(file_path: FilePath) -> Self {
        Self { file_path }
    }

    pub fn open(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let lines = stdin.lock().lines();
        let mut responder = ResponseWriter::new(&mut stdout);

        let file = File::new(self.file_path.clone());
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(file);
        editor.file_read()?;
        let mut status_message = StatusMessage::Empty;
        let mut size: (u16, u16) = (80, 24);
        let mut command_ui = PromptUiState::new();

        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: RpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(err) => {
                    let _ = responder.emit_error(format!("invalid JSON: {}", err));
                    continue;
                }
            };

            let mut record = RequestOutcomeRecord::new();
            let mut context = RequestContext::new(
                &mut editor,
                &mut editor_mode,
                &mut status_message,
                &mut size,
                &mut command_ui,
            );
            record.accept(request, &mut context);
            match record.decision() {
                RequestOutcome::Frame => {
                    let frame = build_frame(
                        &editor,
                        &editor_mode,
                        &status_message,
                        size,
                        if editor_mode.prompt() {
                            Some(command_ui.frame())
                        } else {
                            None
                        },
                    );
                    if let Err(err) = responder.emit_frame(frame) {
                        let _ = responder.emit_error(format!("failed to serialize frame: {}", err));
                    }
                }
                RequestOutcome::Ack(ack) => {
                    if let Err(err) = responder.emit_ack(ack) {
                        let _ = responder.emit_error(format!("failed to serialize ack: {}", err));
                    }
                }
                RequestOutcome::FrameAndAck(ack) => {
                    if let Err(err) = responder.emit_ack(ack) {
                        let _ = responder.emit_error(format!("failed to serialize ack: {}", err));
                        continue;
                    }
                    let frame = build_frame(
                        &editor,
                        &editor_mode,
                        &status_message,
                        size,
                        if editor_mode.prompt() {
                            Some(command_ui.frame())
                        } else {
                            None
                        },
                    );
                    if let Err(err) = responder.emit_frame(frame) {
                        let _ = responder.emit_error(format!("failed to serialize frame: {}", err));
                    }
                }
                RequestOutcome::Quit => break,
                RequestOutcome::Skip => {}
                RequestOutcome::Error(message) => {
                    let _ = responder.emit_error(message);
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub enum RequestOutcome {
    Frame,
    Ack(Ack),
    FrameAndAck(Ack),
    Skip,
    Quit,
    Error(String),
}

struct RequestContext<'a> {
    editor: &'a mut Editor,
    editor_mode: &'a mut EditorMode,
    status: &'a mut StatusMessage,
    size: &'a mut (u16, u16),
    command_ui: &'a mut PromptUiState,
}

impl<'a> RequestContext<'a> {
    fn new(
        editor: &'a mut Editor,
        editor_mode: &'a mut EditorMode,
        status: &'a mut StatusMessage,
        size: &'a mut (u16, u16),
        command_ui: &'a mut PromptUiState,
    ) -> Self {
        Self {
            editor,
            editor_mode,
            status,
            size,
            command_ui,
        }
    }
}

struct RequestOutcomeRecord {
    outcome: RequestOutcome,
}

impl RequestOutcomeRecord {
    fn new() -> Self {
        Self {
            outcome: RequestOutcome::Skip,
        }
    }

    fn decision(&self) -> RequestOutcome {
        self.outcome.clone()
    }
}

struct TextInsertAction {
    text: String,
    signal: FrameSignal,
}

impl TextInsertAction {
    fn apply(&mut self, editor: &mut Editor, status: &mut StatusMessage) {
        let snapshot = editor.snapshot();
        for ch in self.text.chars() {
            editor.char_insert(ch);
        }
        *status = snapshot.status_from(editor, status.clone());
        self.signal = snapshot.frame_signal(editor);
    }

    fn outcome(&self) -> RequestOutcome {
        match self.signal {
            FrameSignal::Frame => RequestOutcome::Frame,
            FrameSignal::Skip => RequestOutcome::Skip,
        }
    }
}

struct TextDeleteAction {
    kind: DeleteKind,
    signal: FrameSignal,
}

impl TextDeleteAction {
    fn apply(&mut self, editor: &mut Editor, status: &mut StatusMessage) {
        let snapshot = editor.snapshot();
        match self.kind {
            DeleteKind::Backspace => editor.backspace_delete(),
            DeleteKind::Under => editor.under_cursor_delete(),
        };
        *status = snapshot.status_from(editor, status.clone());
        self.signal = snapshot.frame_signal(editor);
    }

    fn outcome(&self) -> RequestOutcome {
        match self.signal {
            FrameSignal::Frame => RequestOutcome::Frame,
            FrameSignal::Skip => RequestOutcome::Skip,
        }
    }
}

struct LineBreakAction {
    signal: FrameSignal,
}

impl LineBreakAction {
    fn apply(&mut self, editor: &mut Editor, status: &mut StatusMessage) {
        let snapshot = editor.snapshot();
        editor.line_break();
        *status = snapshot.status_from(editor, status.clone());
        self.signal = snapshot.frame_signal(editor);
    }

    fn outcome(&self) -> RequestOutcome {
        match self.signal {
            FrameSignal::Frame => RequestOutcome::Frame,
            FrameSignal::Skip => RequestOutcome::Skip,
        }
    }
}

struct CursorMoveAction {
    direction: MoveDirection,
    usable_rows: u16,
    signal: FrameSignal,
}

impl CursorMoveAction {
    fn apply(&mut self, editor: &mut Editor) {
        let snapshot = editor.snapshot();
        editor.cursor_move(self.direction, self.usable_rows);
        self.signal = snapshot.frame_signal(editor);
    }

    fn outcome(&self) -> RequestOutcome {
        match self.signal {
            FrameSignal::Frame => RequestOutcome::Frame,
            FrameSignal::Skip => RequestOutcome::Skip,
        }
    }
}

struct PromptUiActionRequest {
    action: PromptUiAction,
    list_rows: usize,
    signal: FrameSignal,
}

impl PromptUiActionRequest {
    fn apply(&mut self, command_ui: &mut PromptUiState) {
        let snapshot = command_ui.snapshot();
        command_ui.apply_action(self.action, self.list_rows);
        let view = command_ui.view();
        self.signal = snapshot.frame_signal(&view);
    }

    fn outcome(&self) -> RequestOutcome {
        match self.signal {
            FrameSignal::Frame => RequestOutcome::Frame,
            FrameSignal::Skip => RequestOutcome::Skip,
        }
    }
}

enum CommandLineRequest {
    Provided(String),
    FromUi,
}

enum CommandPath {
    Missing,
    Provided(String),
}

enum CaseStyle {
    Kebab,
    Camel,
    Snake,
    ScreamingSnake,
    Pascal,
    Train,
    Flat,
}

enum CaseStyleChoice {
    Missing,
    Kebab,
    Camel,
    Snake,
    ScreamingSnake,
    Pascal,
    Train,
    Flat,
    Unknown,
}

enum CommandRequest {
    Save,
    SaveAndQuit,
    Quit,
    Open { path: CommandPath },
    History,
    Case { style: CaseStyle },
    Skip,
}

enum CommandExecutionDecision {
    Allow,
    Block,
}

enum PlaceholderPresence {
    Found,
    Missing,
}

struct PromptInputPlaceholderProbe<'a> {
    line: &'a str,
}

impl<'a> PromptInputPlaceholderProbe<'a> {
    fn presence(&self) -> PlaceholderPresence {
        if self.line.contains('{') && self.line.contains('}') {
            PlaceholderPresence::Found
        } else {
            PlaceholderPresence::Missing
        }
    }
}

struct CommandExecutionGate {
    selection: PromptInputSelection,
    placeholder: PlaceholderPresence,
}

impl CommandExecutionGate {
    fn decision(&self) -> CommandExecutionDecision {
        match self.selection {
            PromptInputSelection::Range { .. } => CommandExecutionDecision::Block,
            PromptInputSelection::None => match self.placeholder {
                PlaceholderPresence::Found => CommandExecutionDecision::Block,
                PlaceholderPresence::Missing => CommandExecutionDecision::Allow,
            },
        }
    }
}

struct CommandText {
    raw: String,
}

impl CommandText {
    fn request(&self) -> CommandRequest {
        let trimmed = self.raw.trim_start_matches(':').trim();
        if trimmed.is_empty() {
            return CommandRequest::Skip;
        }
        let parts = CommandParts::new(trimmed);
        match parts.name.as_str() {
            "s" | "save" => CommandRequest::Save,
            "sq" => CommandRequest::SaveAndQuit,
            "q" | "quit" => CommandRequest::Quit,
            "o" | "open" => {
                let path = if parts.rest.is_empty() {
                    CommandPath::Missing
                } else {
                    CommandPath::Provided(parts.rest.clone())
                };
                CommandRequest::Open { path }
            }
            "history" => CommandRequest::History,
            "case" => {
                let argument = CaseArgument {
                    raw: parts.rest.clone(),
                };
                match argument.style() {
                    CaseStyleChoice::Kebab => CommandRequest::Case {
                        style: CaseStyle::Kebab,
                    },
                    CaseStyleChoice::Camel => CommandRequest::Case {
                        style: CaseStyle::Camel,
                    },
                    CaseStyleChoice::Snake => CommandRequest::Case {
                        style: CaseStyle::Snake,
                    },
                    CaseStyleChoice::ScreamingSnake => CommandRequest::Case {
                        style: CaseStyle::ScreamingSnake,
                    },
                    CaseStyleChoice::Pascal => CommandRequest::Case {
                        style: CaseStyle::Pascal,
                    },
                    CaseStyleChoice::Train => CommandRequest::Case {
                        style: CaseStyle::Train,
                    },
                    CaseStyleChoice::Flat => CommandRequest::Case {
                        style: CaseStyle::Flat,
                    },
                    CaseStyleChoice::Missing | CaseStyleChoice::Unknown => CommandRequest::Skip,
                }
            }
            _ => CommandRequest::Skip,
        }
    }
}

struct CommandParts {
    name: String,
    rest: String,
}

impl CommandParts {
    fn new(line: &str) -> Self {
        let mut split_at = line.len();
        for (idx, ch) in line.char_indices() {
            if ch.is_whitespace() {
                split_at = idx;
                break;
            }
        }
        let (name, rest) = line.split_at(split_at);
        let name = name.to_lowercase();
        let rest = rest.trim_start().to_string();
        Self { name, rest }
    }
}

struct CaseArgument {
    raw: String,
}

impl CaseArgument {
    fn style(&self) -> CaseStyleChoice {
        let raw = self.raw.trim();
        if raw.is_empty() {
            return CaseStyleChoice::Missing;
        }
        let mut split_at = raw.len();
        for (idx, ch) in raw.char_indices() {
            if ch.is_whitespace() {
                split_at = idx;
                break;
            }
        }
        let (token, _) = raw.split_at(split_at);
        match token.to_lowercase().as_str() {
            "kebab" => CaseStyleChoice::Kebab,
            "camel" => CaseStyleChoice::Camel,
            "snake" => CaseStyleChoice::Snake,
            "screaming" => CaseStyleChoice::ScreamingSnake,
            "pascal" => CaseStyleChoice::Pascal,
            "train" => CaseStyleChoice::Train,
            "flat" => CaseStyleChoice::Flat,
            _ => CaseStyleChoice::Unknown,
        }
    }
}

struct CommandExecuteAction {
    line: CommandLineRequest,
    list_rows: usize,
    selection_signal: FrameSignal,
    outcome: RequestOutcome,
}

impl CommandExecuteAction {
    fn apply(
        &mut self,
        editor: &mut Editor,
        editor_mode: &mut EditorMode,
        status: &mut StatusMessage,
        command_ui: &mut PromptUiState,
    ) {
        self.selection_signal = FrameSignal::Skip;
        match &self.line {
            CommandLineRequest::FromUi => {
                let snapshot = command_ui.snapshot();
                command_ui.apply_action(PromptUiAction::SelectFromList, self.list_rows);
                let view = command_ui.view();
                self.selection_signal = snapshot.frame_signal(&view);
            }
            CommandLineRequest::Provided(_) => {}
        }

        let source_line = match &self.line {
            CommandLineRequest::Provided(line) => {
                command_ui.line_overwrite(line.clone());
                line.clone()
            }
            CommandLineRequest::FromUi => command_ui.command_text().to_string(),
        };
        let placeholder = PromptInputPlaceholderProbe {
            line: source_line.as_str(),
        }
        .presence();
        let gate = CommandExecutionGate {
            selection: command_ui.line_selection(),
            placeholder,
        };
        if let CommandExecutionDecision::Block = gate.decision() {
            self.outcome = match self.selection_signal {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            };
            return;
        }
        command_ui.remember_command(&source_line);
        let command = CommandText { raw: source_line }.request();
        self.outcome = match command {
            CommandRequest::Save => {
                let mut saved_path = FilePath::Missing;
                match editor.file_save(&mut saved_path) {
                    Ok(msg) => {
                        *status = StatusMessage::Text { text: msg.clone() };
                        if editor_mode.prompt_command() {
                            command_ui.clear();
                        }
                        let path = editor.file_path();
                        editor_mode.transition(RequestEditorMode::Normal, &path);
                        editor.visual_clear();
                        let ack =
                            Ack::new(AckKind::Save, StatusMessage::Text { text: msg }, saved_path);
                        RequestOutcome::FrameAndAck(ack)
                    }
                    Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                }
            }
            CommandRequest::SaveAndQuit => {
                let mut saved_path = FilePath::Missing;
                match editor.file_save(&mut saved_path) {
                    Ok(msg) => {
                        *status = StatusMessage::Text { text: msg.clone() };
                        RequestOutcome::Quit
                    }
                    Err(err) => RequestOutcome::Error(format!("save failed: {}", err)),
                }
            }
            CommandRequest::Quit => RequestOutcome::Quit,
            CommandRequest::Open { path } => match path {
                CommandPath::Provided(path) => {
                    let mut new_file = File::new(FilePath::Provided { path });
                    if let Err(err) = new_file.read() {
                        RequestOutcome::Error(format!("open failed: {}", err))
                    } else {
                        *editor_mode = EditorMode::new();
                        *editor = Editor::new(new_file);
                        *status = StatusMessage::Empty;
                        if editor_mode.prompt_command() {
                            command_ui.clear();
                        }
                        let path = editor.file_path();
                        editor_mode.transition(RequestEditorMode::Normal, &path);
                        let ack = Ack::new(
                            AckKind::Open,
                            StatusMessage::Text {
                                text: String::from("opened"),
                            },
                            editor.file_path(),
                        );
                        RequestOutcome::FrameAndAck(ack)
                    }
                }
                CommandPath::Missing => match editor.file_path() {
                    FilePath::Provided { path } => {
                        let mut new_file = File::new(FilePath::Provided { path });
                        if let Err(err) = new_file.read() {
                            RequestOutcome::Error(format!("reload failed: {}", err))
                        } else {
                            *editor_mode = EditorMode::new();
                            *editor = Editor::new(new_file);
                            *status = StatusMessage::Empty;
                            if editor_mode.prompt_command() {
                                command_ui.clear();
                            }
                            let path = editor.file_path();
                            editor_mode.transition(RequestEditorMode::Normal, &path);
                            let ack = Ack::new(
                                AckKind::Open,
                                StatusMessage::Text {
                                    text: String::from("reloaded"),
                                },
                                editor.file_path(),
                            );
                            RequestOutcome::FrameAndAck(ack)
                        }
                    }
                    FilePath::Missing => {
                        RequestOutcome::Error(String::from("reload failed: no file path"))
                    }
                },
            },
            CommandRequest::History => match command_ui.history() {
                FilePath::Provided { path } => {
                    let mut new_file = File::new(FilePath::Provided { path });
                    if let Err(err) = new_file.read() {
                        RequestOutcome::Error(format!("history open failed: {}", err))
                    } else {
                        *editor_mode = EditorMode::new();
                        *editor = Editor::new(new_file);
                        *status = StatusMessage::Empty;
                        if editor_mode.prompt_command() {
                            command_ui.clear();
                        }
                        let path = editor.file_path();
                        editor_mode.transition(RequestEditorMode::Normal, &path);
                        let ack = Ack::new(
                            AckKind::Open,
                            StatusMessage::Text {
                                text: String::from("opened"),
                            },
                            editor.file_path(),
                        );
                        RequestOutcome::FrameAndAck(ack)
                    }
                }
                FilePath::Missing => RequestOutcome::Error(String::from("history file missing")),
            },
            CommandRequest::Case { style } => {
                match style {
                    CaseStyle::Kebab => editor.selection_case_kebab(),
                    CaseStyle::Camel => editor.selection_case_camel(),
                    CaseStyle::Snake => editor.selection_case_snake(),
                    CaseStyle::ScreamingSnake => editor.selection_case_screaming_snake(),
                    CaseStyle::Pascal => editor.selection_case_pascal(),
                    CaseStyle::Train => editor.selection_case_train(),
                    CaseStyle::Flat => editor.selection_case_flat(),
                }
                if editor_mode.prompt_command() {
                    command_ui.clear();
                }
                let path = editor.file_path();
                editor_mode.transition(RequestEditorMode::Normal, &path);
                RequestOutcome::Frame
            }
            CommandRequest::Skip => match self.selection_signal {
                FrameSignal::Frame => RequestOutcome::Frame,
                FrameSignal::Skip => RequestOutcome::Skip,
            },
        };
    }

    fn finish(self) -> RequestOutcome {
        self.outcome
    }
}

impl RequestOutcomeRecord {
    fn accept(&mut self, request: RpcRequest, context: &mut RequestContext<'_>) {
        let editor = &mut *context.editor;
        let editor_mode = &mut *context.editor_mode;
        let status = &mut *context.status;
        let size = &mut *context.size;
        let command_ui = &mut *context.command_ui;

        self.outcome = match request {
            RpcRequest::EditorResize {
                cols,
                rows,
                suppress_frame,
            } => {
                let prev = *size;
                *size = (cols, rows);
                if editor_mode.prompt() {
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
                    RequestOutcome::Error(format!("open failed: {}", err))
                } else {
                    *editor_mode = EditorMode::new();
                    *editor = Editor::new(new_file);
                    *status = StatusMessage::Empty;
                    let ack = Ack::new(
                        AckKind::Open,
                        StatusMessage::Text {
                            text: String::from("opened"),
                        },
                        editor.file_path(),
                    );
                    RequestOutcome::FrameAndAck(ack)
                }
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
                        *editor_mode = EditorMode::new();
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
                let mut action = TextInsertAction {
                    text,
                    signal: FrameSignal::Skip,
                };
                action.apply(editor, status);
                action.outcome()
            }
            RpcRequest::TextDelete { kind } => {
                let mut action = TextDeleteAction {
                    kind,
                    signal: FrameSignal::Skip,
                };
                action.apply(editor, status);
                action.outcome()
            }
            RpcRequest::LineBreak => {
                let mut action = LineBreakAction {
                    signal: FrameSignal::Skip,
                };
                action.apply(editor, status);
                action.outcome()
            }
            RpcRequest::CursorMove { direction } => {
                let usable_rows = size.1.saturating_sub(3);
                let mut action = CursorMoveAction {
                    direction,
                    usable_rows,
                    signal: FrameSignal::Skip,
                };
                action.apply(editor);
                action.outcome()
            }
            RpcRequest::CommandUi { action } => {
                if !editor_mode.prompt() {
                    RequestOutcome::Skip
                } else {
                    let list_rows = command_list_rows(*size);
                    let mut request = PromptUiActionRequest {
                        action,
                        list_rows,
                        signal: FrameSignal::Skip,
                    };
                    request.apply(command_ui);
                    request.outcome()
                }
            }
            RpcRequest::CommandExecute { line } => {
                if !editor_mode.prompt_command() {
                    RequestOutcome::Skip
                } else {
                    let list_rows = command_list_rows(*size);
                    let line = match line {
                        Some(line) => CommandLineRequest::Provided(line),
                        None => CommandLineRequest::FromUi,
                    };
                    let mut request = CommandExecuteAction {
                        line,
                        list_rows,
                        selection_signal: FrameSignal::Skip,
                        outcome: RequestOutcome::Skip,
                    };
                    request.apply(editor, editor_mode, status, command_ui);
                    request.finish()
                }
            }
            RpcRequest::ModeSet { mode: new_mode } => {
                let prev_mode = editor_mode.mode();
                let prev_cursor = editor.cursor_position();
                let path = editor.file_path();
                editor_mode.transition(new_mode, &path);
                let next_mode = editor_mode.mode();
                if matches!(next_mode, EditorModeState::Edit)
                    && !matches!(prev_mode, EditorModeState::Edit)
                {
                    editor.snap_cursor_to_tab_start();
                }
                if next_mode == EditorModeState::PromptCommand
                    && prev_mode != EditorModeState::PromptCommand
                {
                    command_ui.prompt_command_for(editor.command_scope());
                } else if next_mode == EditorModeState::PromptKeymap
                    && prev_mode != EditorModeState::PromptKeymap
                {
                    command_ui.prompt_keymap();
                } else if matches!(
                    prev_mode,
                    EditorModeState::PromptCommand | EditorModeState::PromptKeymap
                ) && !matches!(
                    next_mode,
                    EditorModeState::PromptCommand | EditorModeState::PromptKeymap
                ) {
                    command_ui.clear();
                }
                if matches!(next_mode, EditorModeState::Normal | EditorModeState::Edit) {
                    editor.visual_clear();
                }
                if next_mode == EditorModeState::Visual
                    && !matches!(prev_mode, EditorModeState::Visual)
                {
                    editor.visual_begin();
                }
                if next_mode != prev_mode || editor.cursor_position() != prev_cursor {
                    RequestOutcome::Frame
                } else {
                    RequestOutcome::Skip
                }
            }
            RpcRequest::StateGet => RequestOutcome::Frame,
            RpcRequest::EditorQuit => RequestOutcome::Quit,
        };
    }
}

pub fn build_frame(
    editor: &Editor,
    editor_mode: &EditorMode,
    status: &StatusMessage,
    size: (u16, u16),
    command_ui: Option<PromptUiFrame>,
) -> Frame {
    let usable_rows = size.1.saturating_sub(3);
    let view = editor.view_with_scroll(size.0, usable_rows);
    let rows = editor.rows_render(&view, size.0, usable_rows);
    let selection = editor.selection_frame(&view, size.0, usable_rows);

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
    let status = editor.message_lock().or(status);
    let total_rows = view.file_ref().line_total();
    let status_position = StatusPosition::new(view.cursor_column(), view.cursor_row(), total_rows);

    Frame::new(
        match editor_mode.mode() {
            EditorModeState::Normal => FrameEditorMode::Normal,
            EditorModeState::Edit => FrameEditorMode::Edit,
            EditorModeState::Visual => FrameEditorMode::Visual,
            EditorModeState::PromptCommand => FrameEditorMode::PromptCommand,
            EditorModeState::PromptKeymap => FrameEditorMode::PromptKeymap,
        },
        Cursor::new(cursor_col, cursor_row),
        rows,
        status,
        status_position,
        view.file_ref().path(),
        size,
        command_ui,
        selection,
        ProtocolVersion::current(),
    )
}

struct ResponseWriter<'a> {
    stdout: &'a mut dyn Write,
}

impl<'a> ResponseWriter<'a> {
    fn new(stdout: &'a mut dyn Write) -> Self {
        Self { stdout }
    }

    fn emit_frame(&mut self, frame: Frame) -> io::Result<()> {
        self.emit(RpcResponse::Frame(frame))
    }

    fn emit_ack(&mut self, ack: Ack) -> io::Result<()> {
        self.emit(RpcResponse::Ack(ack))
    }

    fn emit_error(&mut self, message: String) -> io::Result<()> {
        self.emit(RpcResponse::Error { message })
    }

    fn emit(&mut self, response: RpcResponse) -> io::Result<()> {
        serde_json::to_writer(&mut *self.stdout, &response)?;
        self.stdout.write_all(b"\n")?;
        self.stdout.flush()
    }
}

fn command_list_rows(size: (u16, u16)) -> usize {
    size.1
        .saturating_sub(3) // command line + status line + help line
        .saturating_sub(3) as usize // blank + header + divider rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::CursorPosition;
    use std::fs;
    use vimrust_protocol::{CommandUiAccess, CursorSink, FrameRowSink, MoveDirection};

    struct RequestHarness<'a> {
        record: RequestOutcomeRecord,
        context: RequestContext<'a>,
    }

    impl<'a> RequestHarness<'a> {
        fn new(
            editor: &'a mut Editor,
            editor_mode: &'a mut EditorMode,
            status: &'a mut StatusMessage,
            size: &'a mut (u16, u16),
            command_ui: &'a mut PromptUiState,
        ) -> Self {
            Self {
                record: RequestOutcomeRecord::new(),
                context: RequestContext::new(editor, editor_mode, status, size, command_ui),
            }
        }

        fn accept(&mut self, request: RpcRequest) {
            self.record.accept(request, &mut self.context);
        }

        fn decision(&self) -> RequestOutcome {
            self.record.decision()
        }
    }

    struct RowsProbe {
        rows: Vec<String>,
    }

    impl RowsProbe {
        fn new() -> Self {
            Self { rows: Vec::new() }
        }

        fn expect_row(&self, index: u16, expected: &str) {
            let found = self
                .rows
                .get(index as usize)
                .map(String::as_str)
                .unwrap_or("");
            assert_eq!(found, expected);
        }
    }

    impl FrameRowSink for RowsProbe {
        fn paint_row(&mut self, index: u16, row: &str, _selection: vimrust_protocol::RowSelection) {
            if self.rows.len() <= index as usize {
                self.rows
                    .resize_with(index.saturating_add(1) as usize, String::new);
            }
            self.rows[index as usize] = row.to_string();
        }
    }

    struct CursorProbe {
        column: u16,
        row: u16,
        placed: bool,
    }

    impl CursorProbe {
        fn new() -> Self {
            Self {
                column: 0,
                row: 0,
                placed: false,
            }
        }

        fn expect_position(&self, column: u16, row: u16) {
            assert!(self.placed);
            assert_eq!(self.column, column);
            assert_eq!(self.row, row);
        }
    }

    impl CursorSink for CursorProbe {
        fn place(&mut self, column: u16, row: u16) {
            self.column = column;
            self.row = row;
            self.placed = true;
        }
    }

    #[test]
    fn insert_request_updates_rows() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::TextInsert {
            text: "hi".to_string(),
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Frame));

        let frame = build_frame(&editor, &editor_mode, &status, size, None);
        let mut rows = RowsProbe::new();
        frame.paint_rows(size.1.saturating_sub(3), &mut rows);
        rows.expect_row(0, "hi");
    }

    #[test]
    fn line_break_splits_line_at_cursor() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::from("hello world")]);
        editor.cursor_position_store(CursorPosition::new(5, 0));
        let mut status = StatusMessage::Empty;
        let mut size = (20, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::LineBreak);
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Frame));

        let frame = build_frame(&editor, &editor_mode, &status, size, None);
        let mut rows = RowsProbe::new();
        frame.paint_rows(size.1.saturating_sub(3), &mut rows);
        rows.expect_row(0, "hello");
        rows.expect_row(1, " world");
    }

    #[test]
    fn noop_cursor_move_skips_frame() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::new()]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CursorMove {
            direction: MoveDirection::Left,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn resize_with_suppress_frame_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::EditorResize {
            cols: 20,
            rows: 40,
            suppress_frame: true,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert_eq!(size, (20, 40));
    }

    #[test]
    fn resize_same_size_without_suppress_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::EditorResize {
            cols: 10,
            rows: 5,
            suppress_frame: false,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert_eq!(size, (10, 5));
    }

    #[test]
    fn command_ui_request_updates_state_and_frame() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (20, 8);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Frame));

        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'x' },
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Frame));

        let frame = build_frame(
            &editor,
            &editor_mode,
            &status,
            size,
            Some(command_ui.frame()),
        );
        let command_ui_frame = match frame.command_ui() {
            CommandUiAccess::Available(command_ui) => command_ui,
            CommandUiAccess::Missing => panic!("expected command ui frame"),
        };
        assert_eq!(command_ui_frame.command_text(), ":x");
        assert_eq!(command_ui_frame.cursor_column(), 2);
    }

    #[test]
    fn command_ui_outside_command_mode_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'x' },
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn noop_delete_under_cursor_skips_frame() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::new()]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::TextDelete {
            kind: DeleteKind::Under,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn file_open_missing_path_returns_error() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        let path = std::env::temp_dir().join("vimrust_missing_file.txt");
        let _ = fs::remove_file(&path);

        harness.accept(RpcRequest::FileOpen {
            path: path.to_string_lossy().to_string(),
        });
        let outcome = harness.decision();
        match outcome {
            RequestOutcome::Error(message) => {
                assert!(message.starts_with("open failed:"));
            }
            _ => panic!("expected error outcome"),
        }
    }

    #[test]
    fn save_as_writes_file_and_updates_status() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![String::from("hello")]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        let path = std::env::temp_dir().join("vimrust_rpc_test.txt");
        let _ = fs::remove_file(&path);

        harness.accept(RpcRequest::FileSaveAs {
            path: path.to_string_lossy().to_string(),
        });
        let outcome = harness.decision();
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

        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":s".to_string()),
        });
        let outcome = harness.decision();
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert!(matches!(editor_mode.mode(), EditorModeState::Normal));
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

        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 's' },
        });

        harness.accept(RpcRequest::CommandExecute { line: None });
        let outcome = harness.decision();
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert!(matches!(editor_mode.mode(), EditorModeState::Normal));
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

        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });

        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::MoveSelectionDown,
        });

        harness.accept(RpcRequest::CommandExecute { line: None });
        let outcome = harness.decision();
        if let RequestOutcome::FrameAndAck(ack) = outcome {
            assert_eq!(ack.kind(), AckKind::Save);
            assert_eq!(
                status,
                StatusMessage::Text {
                    text: String::from("saved"),
                }
            );
            assert!(matches!(editor_mode.mode(), EditorModeState::Normal));
        } else {
            panic!("expected frame and ack");
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn command_execute_list_placeholder_skips_execution_for_open_query() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });

        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'o' },
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'p' },
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'e' },
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 'n' },
        });

        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::MoveSelectionDown,
        });

        harness.accept(RpcRequest::CommandExecute { line: None });
        let outcome = harness.decision();

        match outcome {
            RequestOutcome::Ack(_) | RequestOutcome::FrameAndAck(_) | RequestOutcome::Quit => {
                panic!("expected command execution to be skipped");
            }
            RequestOutcome::Frame | RequestOutcome::Skip | RequestOutcome::Error(_) => {}
        }
        assert!(editor_mode.prompt_command());
    }

    #[test]
    fn command_execute_placeholder_line_skips_execution() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":o {filename}".to_string()),
        });
        let outcome = harness.decision();

        match outcome {
            RequestOutcome::Ack(_) | RequestOutcome::FrameAndAck(_) | RequestOutcome::Quit => {
                panic!("expected command execution to be skipped");
            }
            RequestOutcome::Frame | RequestOutcome::Skip | RequestOutcome::Error(_) => {}
        }
        assert!(editor_mode.prompt_command());
    }

    #[test]
    fn command_execute_quit_requests_quit_outcome() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":q".to_string()),
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Quit));
    }

    #[test]
    fn command_execute_open_reads_file_and_emits_ack() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );
        let path = std::env::temp_dir().join("vimrust_rpc_command_open.txt");
        let _ = fs::write(&path, "hello");

        harness.accept(RpcRequest::CommandExecute {
            line: Some(format!(":o {}", path.to_string_lossy())),
        });
        let outcome = harness.decision();

        assert!(matches!(outcome, RequestOutcome::FrameAndAck(_)));
        assert_eq!(
            editor.file_path(),
            FilePath::Provided {
                path: path.to_string_lossy().to_string()
            }
        );
    }

    #[test]
    fn command_execute_reload_reads_current_file() {
        let path = std::env::temp_dir().join("vimrust_rpc_command_reload.txt");
        let _ = fs::write(&path, "disk");
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("memory")]);
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":o".to_string()),
        });
        let outcome = harness.decision();

        assert!(matches!(outcome, RequestOutcome::FrameAndAck(_)));
        assert_eq!(editor.file_lines_snapshot(), vec![String::from("disk")]);
    }

    #[test]
    fn command_execute_unknown_command_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptCommand, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":unknown".to_string()),
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert!(editor_mode.prompt_command());
    }

    #[test]
    fn command_execute_outside_command_mode_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":q".to_string()),
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn command_execute_in_keymap_prompt_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let path = editor.file_path();
        editor_mode.transition(RequestEditorMode::PromptKeymap, &path);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::CommandExecute {
            line: Some(":q".to_string()),
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert!(matches!(editor_mode.mode(), EditorModeState::PromptKeymap));
    }

    #[test]
    fn mode_set_same_mode_skips() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::Normal,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
    }

    #[test]
    fn mode_set_edit_skips_on_locked_file() {
        let path = std::env::temp_dir().join("vimrust_mode_set_locked.txt");
        let _ = fs::write(&path, "locked");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).expect("set permissions");

        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::Edit,
        });
        let outcome = harness.decision();
        assert!(matches!(outcome, RequestOutcome::Skip));
        assert!(matches!(editor_mode.mode(), EditorModeState::Normal));

        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_readonly(false);
        let _ = fs::set_permissions(&path, permissions);
    }

    #[test]
    fn command_ui_frame_includes_line_and_selection() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptCommand,
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::InsertChar { ch: 's' },
        });
        harness.accept(RpcRequest::CommandUi {
            action: PromptUiAction::MoveSelectionDown,
        });

        let frame = build_frame(
            &editor,
            &editor_mode,
            &status,
            size,
            Some(command_ui.frame()),
        );
        let command_ui_frame = match frame.command_ui() {
            CommandUiAccess::Available(command_ui) => command_ui,
            CommandUiAccess::Missing => panic!("expected command ui frame"),
        };
        assert_eq!(command_ui_frame.command_text(), ":s");
        assert_eq!(command_ui_frame.cursor_column(), 2);
        assert_eq!(command_ui_frame.line_focus(), false);
        assert!(command_ui_frame.selected_item().is_some());
        assert!(!command_ui_frame.command_items().is_empty());
    }

    #[test]
    fn prompt_keymap_frame_includes_keymap_entries() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (20, 10);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::ModeSet {
            mode: RequestEditorMode::PromptKeymap,
        });

        let frame = build_frame(
            &editor,
            &editor_mode,
            &status,
            size,
            Some(command_ui.frame()),
        );
        let command_ui_frame = match frame.command_ui() {
            CommandUiAccess::Available(command_ui) => command_ui,
            CommandUiAccess::Missing => panic!("expected command ui frame"),
        };
        assert_eq!(command_ui_frame.command_text(), ";");
        let items = command_ui_frame.command_items();
        let mut found = false;
        let mut idx = 0usize;
        while idx < items.len() {
            if items[idx].label() == "q"
                && matches!(items[idx].mode(), vimrust_protocol::PromptMode::Normal)
            {
                found = true;
                break;
            }
            idx = idx.saturating_add(1);
        }
        assert!(found);
    }

    #[test]
    fn frame_cursor_positions_respect_offsets() {
        let editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        editor.file_lines_replace(vec![
            String::from("aaa"),
            String::from("bbb"),
            String::from("ccc"),
            String::from("ddd"),
        ]);
        editor.cursor_position_store(CursorPosition::new(5, 3));

        let status = StatusMessage::Empty;
        let size = (10, 6);

        let frame = build_frame(&editor, &editor_mode, &status, size, None);
        let mut cursor = CursorProbe::new();
        frame.cursor().place_on(&mut cursor);
        cursor.expect_position(5, 3);
    }

    #[test]
    fn mode_transitions_toggle_command_ui_frame() {
        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Missing));
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        {
            let mut harness = RequestHarness::new(
                &mut editor,
                &mut editor_mode,
                &mut status,
                &mut size,
                &mut command_ui,
            );
            harness.accept(RpcRequest::ModeSet {
                mode: RequestEditorMode::PromptCommand,
            });
        }
        let frame_command = build_frame(
            &editor,
            &editor_mode,
            &status,
            size,
            Some(command_ui.frame()),
        );
        assert!(matches!(
            frame_command.mode(),
            FrameEditorMode::PromptCommand
        ));
        match frame_command.command_ui() {
            CommandUiAccess::Available(_) => {}
            CommandUiAccess::Missing => panic!("expected command ui frame"),
        }

        {
            let mut harness = RequestHarness::new(
                &mut editor,
                &mut editor_mode,
                &mut status,
                &mut size,
                &mut command_ui,
            );
            harness.accept(RpcRequest::ModeSet {
                mode: RequestEditorMode::Normal,
            });
        }
        let frame_normal = build_frame(&editor, &editor_mode, &status, size, None);
        assert!(matches!(frame_normal.mode(), FrameEditorMode::Normal));
        match frame_normal.command_ui() {
            CommandUiAccess::Available(_) => panic!("expected command ui to be missing"),
            CommandUiAccess::Missing => {}
        }
    }

    #[test]
    fn save_on_existing_path_emits_ack_without_frame() {
        let path = std::env::temp_dir().join("vimrust_rpc_save_exists.txt");
        let _ = fs::write(&path, "existing");

        let mut editor_mode = EditorMode::new();
        let mut editor = Editor::new(File::new(FilePath::Provided {
            path: path.to_string_lossy().to_string(),
        }));
        editor.file_lines_replace(vec![String::from("changed")]);
        let mut status = StatusMessage::Empty;
        let mut size = (10, 5);
        let mut command_ui = PromptUiState::new();
        let mut harness = RequestHarness::new(
            &mut editor,
            &mut editor_mode,
            &mut status,
            &mut size,
            &mut command_ui,
        );

        harness.accept(RpcRequest::FileSave);
        let outcome = harness.decision();
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
