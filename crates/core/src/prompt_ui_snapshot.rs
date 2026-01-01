use crate::{frame_signal::FrameSignal, prompt_ui_state::CommandUiView};
use vimrust_protocol::CommandLineSelection;

pub struct CommandUiSnapshot {
    command_text: String,
    cursor_column: u16,
    line_selection: CommandLineSelection,
    focus_on_list: bool,
    selection: Option<usize>,
    scroll_offset: usize,
}

impl CommandUiSnapshot {
    pub(crate) fn new(
        command_text: String,
        cursor_column: u16,
        line_selection: CommandLineSelection,
        focus_on_list: bool,
        selection: Option<usize>,
        scroll_offset: usize,
    ) -> Self {
        Self {
            command_text,
            cursor_column,
            line_selection,
            focus_on_list,
            selection,
            scroll_offset,
        }
    }

    pub fn frame_signal(&self, view: &CommandUiView<'_>) -> FrameSignal {
        let same_text = self.command_text == view.text;
        let same_cursor = self.cursor_column == view.cursor;
        let same_line_selection = self.line_selection == view.selection;
        let same_focus = self.focus_on_list == view.focus_on_list;
        let same_selection = self.selection == view.selection_index;
        let same_scroll = self.scroll_offset == view.scroll_offset;
        if same_text
            && same_cursor
            && same_line_selection
            && same_focus
            && same_selection
            && same_scroll
        {
            FrameSignal::Skip
        } else {
            FrameSignal::Frame
        }
    }
}
