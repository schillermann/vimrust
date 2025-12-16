use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode};

mod buffer;
mod command_line;
mod command_list;
mod editor;
mod file;
mod status_line;
mod terminal;
mod ui;

use command_line::CommandLine;
use command_list::CommandList;
use editor::Editor;
use file::File;
use terminal::Terminal;
use ui::Ui;

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
    let file_path = env::args().nth(1);
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

    let result: io::Result<()> = {
        let mut ui = Ui::new(terminal, &mut editor, &mut command_line, &mut command_list);
        ui.set_mode(EditorMode::Normal)?;
        ui.editor().file_read()?;
        ui.terminal_update_size()?;

        loop {
            ui.render(&file_path)?;

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
                                    ui.mode_edit_enter()?;
                                }
                                KeyCode::Char('s') => {
                                    ui.file_save(&mut file_path);
                                }
                                KeyCode::Char(':') => {
                                    ui.mode_command_enter()?;
                                }
                                key_code => {
                                    let usable_rows = ui.editor_view_rows();
                                    ui.editor().cursor_move(key_code, usable_rows);
                                }
                            },
                            EditorMode::Edit => match key_event.code {
                                KeyCode::Esc => {
                                    ui.mode_normal_enter()?;
                                }
                                KeyCode::Delete => {
                                    ui.editor().delete_under_cursor();
                                }
                                KeyCode::Backspace => {
                                    ui.editor().delete_backspace();
                                }
                                KeyCode::Char(ch) => {
                                    ui.editor().insert_char(ch);
                                }
                                _ => {}
                            },
                            EditorMode::Command => match key_event.code {
                                KeyCode::Esc => {
                                    ui.mode_command_exit()?;
                                }
                                KeyCode::Enter => {
                                    ui.command_list_enter_select();
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
