use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod command_line;
mod command_list;
mod editor;
mod status_line;
mod terminal;

use command_list::filter_commands;
use editor::Editor;
use terminal::Terminal;

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

const DEFAULT_STATUS: &str = "| e: edit | Esc: normal | s: save | q: quit";

fn main() -> io::Result<()> {
    let file_path = env::args().nth(1);
    let mut terminal = Terminal::new()?;
    let mut editor = Editor::new();
    let result = run(&mut terminal, &mut editor, file_path);
    terminal.cleanup();
    result
}

fn update_status(current: &mut String, needs_refresh: &mut bool, message: String) {
    if *current != message {
        *current = message;
        *needs_refresh = true;
    }
}

fn run(
    terminal: &mut Terminal,
    editor: &mut Editor,
    mut file_path: Option<String>,
) -> io::Result<()> {
    let result: io::Result<()> = {
        if let Some(path) = file_path.clone() {
            editor.open(path)?;
        }
        editor.ensure_minimum_line();

        let mut terminal_size = terminal.size();
        let mut mode = EditorMode::Normal;
        let mut status_message = String::from(DEFAULT_STATUS);
        let mut needs_refresh = true;
        let mut command_line = String::new();
        let mut command_cursor_x: u16 = 0;
        let mut command_selected_index: usize = 0;
        let mut command_scroll_offset: usize = 0;
        let mut command_focus_on_list: bool = false;
        terminal.set_cursor_style(&mode)?;

        loop {
            if needs_refresh {
                terminal.render_frame(
                    editor,
                    &mode,
                    &file_path,
                    &command_line,
                    command_cursor_x,
                    command_selected_index,
                    command_scroll_offset,
                    command_focus_on_list,
                )?;
                needs_refresh = false;
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        needs_refresh = true;
                        match mode {
                            EditorMode::Normal => match key_event.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('e') => {
                                    mode = EditorMode::Edit;
                                    terminal.set_cursor_style(&mode)?;
                                    editor.snap_cursor_to_tab_start();
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Char('s') => match editor.save(&mut file_path) {
                                    Ok(msg) => {
                                        update_status(&mut status_message, &mut needs_refresh, msg)
                                    }
                                    Err(err) => update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        format!("Error saving: {}", err),
                                    ),
                                },
                                KeyCode::Char(':') => {
                                    mode = EditorMode::Command;
                                    command_line.clear();
                                    command_line.push(':');
                                    command_cursor_x = 1;
                                    command_selected_index = 0;
                                    command_scroll_offset = 0;
                                    command_focus_on_list = false;
                                    terminal.set_cursor_style(&mode)?;
                                }
                                key_code => {
                                    let usable_rows = terminal_size.1.saturating_sub(2);
                                    editor.move_cursor(key_code, usable_rows)?;
                                }
                            },
                            EditorMode::Edit => match key_event.code {
                                KeyCode::Esc => {
                                    mode = EditorMode::Normal;
                                    terminal.set_cursor_style(&mode)?;
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Delete => {
                                    editor.delete_under_cursor();
                                }
                                KeyCode::Backspace => {
                                    editor.delete_backspace();
                                }
                                KeyCode::Char(ch) => {
                                    editor.insert_char(ch);
                                }
                                _ => {}
                            },
                            EditorMode::Command => match key_event.code {
                                KeyCode::Esc => {
                                    mode = EditorMode::Normal;
                                    terminal.set_cursor_style(&mode)?;
                                    command_line.clear();
                                    command_cursor_x = 0;
                                    command_selected_index = 0;
                                    command_scroll_offset = 0;
                                    command_focus_on_list = false;
                                    update_status(
                                        &mut status_message,
                                        &mut needs_refresh,
                                        String::from(DEFAULT_STATUS),
                                    );
                                }
                                KeyCode::Enter => {
                                    let matches = filter_commands(&command_line);
                                    if command_focus_on_list && !matches.is_empty() {
                                        let index = command_selected_index.min(matches.len() - 1);
                                        if let Some(entry) = matches.get(index) {
                                            command_line = format!(":{}", entry.name);
                                            command_cursor_x = command_line.len() as u16;
                                            command_selected_index = 0;
                                            command_scroll_offset = 0;
                                            command_focus_on_list = false;
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if command_cursor_x > 0 {
                                        let delete_at = command_cursor_x.saturating_sub(1) as usize;
                                        if delete_at < command_line.len() {
                                            command_line.remove(delete_at);
                                            command_cursor_x = command_cursor_x.saturating_sub(1);
                                            command_selected_index = 0;
                                            command_scroll_offset = 0;
                                            command_focus_on_list = false;
                                        }
                                    }
                                }
                                KeyCode::Delete => {
                                    let delete_at = command_cursor_x as usize;
                                    if delete_at < command_line.len() {
                                        command_line.remove(delete_at);
                                        command_selected_index = 0;
                                        command_scroll_offset = 0;
                                        command_focus_on_list = false;
                                    }
                                    command_cursor_x =
                                        command_cursor_x.min(command_line.len() as u16);
                                }
                                KeyCode::Left => {
                                    if command_cursor_x > 0 {
                                        command_cursor_x = command_cursor_x.saturating_sub(1);
                                    }
                                    command_focus_on_list = false;
                                }
                                KeyCode::Right => {
                                    let limit = command_line.len() as u16;
                                    if command_cursor_x < limit {
                                        command_cursor_x = command_cursor_x.saturating_add(1);
                                    }
                                    command_focus_on_list = false;
                                }
                                KeyCode::Home => {
                                    command_cursor_x = 0;
                                    command_focus_on_list = false;
                                }
                                KeyCode::End => {
                                    command_cursor_x = command_line.len() as u16;
                                    command_focus_on_list = false;
                                }
                                KeyCode::Up | KeyCode::Down => {
                                    let matches = filter_commands(&command_line);
                                    if matches.is_empty() {
                                        command_selected_index = 0;
                                        command_scroll_offset = 0;
                                        command_focus_on_list = false;
                                    } else {
                                        command_focus_on_list = true;
                                        let max_index = matches.len().saturating_sub(1);
                                        if matches!(key_event.code, KeyCode::Up) {
                                            if command_selected_index > 0 {
                                                command_selected_index =
                                                    command_selected_index.saturating_sub(1);
                                            }
                                        } else if command_selected_index < max_index {
                                            command_selected_index =
                                                command_selected_index.saturating_add(1);
                                        }
                                        let list_rows =
                                            terminal_size.1.saturating_sub(2).saturating_sub(3)
                                                as usize;
                                        if list_rows > 0 {
                                            if command_selected_index < command_scroll_offset {
                                                command_scroll_offset = command_selected_index;
                                            } else if command_selected_index
                                                >= command_scroll_offset.saturating_add(list_rows)
                                            {
                                                command_scroll_offset = command_selected_index
                                                    .saturating_sub(list_rows)
                                                    .saturating_add(1);
                                            }
                                        } else {
                                            command_scroll_offset = 0;
                                        }
                                    }
                                }
                                KeyCode::Char(ch) => {
                                    let insert_at = command_cursor_x as usize;
                                    if insert_at <= command_line.len() {
                                        command_line.insert(insert_at, ch);
                                        command_cursor_x = command_cursor_x.saturating_add(1);
                                        command_selected_index = 0;
                                        command_scroll_offset = 0;
                                        command_focus_on_list = false;
                                    }
                                }
                                _ => {}
                            },
                        }
                    }
                    Event::Resize(_, _) => {
                        terminal.update_size()?;
                        terminal_size = terminal.size();
                        needs_refresh = true;
                    }
                    _ => {}
                }
            } else {
                // periodic tasks / redraw can go here
            }
        }
        Ok(())
    };

    result
}
