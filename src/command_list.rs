use std::io;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};

pub struct CommandEntry {
    pub name: &'static str,
    pub description: &'static str,
}

static COMMANDS: &[CommandEntry] = &[
    CommandEntry {
        name: "s",
        description: "Save the current buffer",
    },
    CommandEntry {
        name: "save",
        description: "Save the current buffer",
    },
    CommandEntry {
        name: "q",
        description: "Quit the editor",
    },
    CommandEntry {
        name: "quit",
        description: "Quit the editor",
    },
    CommandEntry {
        name: "sq",
        description: "Save and quit",
    },
    CommandEntry {
        name: "o filename",
        description: "Open a file",
    },
    CommandEntry {
        name: "open filename",
        description: "Open a file",
    },
];

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut query_chars = query.chars();
    let mut current = match query_chars.next() {
        Some(ch) => ch,
        None => return true,
    };

    for cand in candidate.chars() {
        if cand == current {
            if let Some(next) = query_chars.next() {
                current = next;
            } else {
                return true;
            }
        }
    }

    false
}

fn matched_indices(query: &str, candidate: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut positions = Vec::new();
    let mut q_iter = query.chars().peekable();
    let mut q_index = 0usize;
    for (idx, ch) in candidate.chars().enumerate() {
        if let Some(&qch) = q_iter.peek() {
            if ch.to_ascii_lowercase() == qch.to_ascii_lowercase() {
                positions.push(idx);
                q_iter.next();
                q_index += 1;
                if q_index >= query.len() {
                    break;
                }
            }
        } else {
            break;
        }
    }
    positions
}

fn command_query_from_input(command_line: &str) -> String {
    let trimmed = command_line.trim_start_matches(':').trim();
    trimmed.to_lowercase()
}

fn queue_highlighted(
    buffer: &mut Vec<u8>,
    text: &str,
    match_indices: &[usize],
    default_fg: Option<Color>,
    highlight_fg: Color,
    keep_background: bool,
) -> io::Result<()> {
    let mut match_iter = match_indices.iter().copied();
    let mut next_match = match_iter.next();

    if let Some(color) = default_fg {
        queue!(buffer, SetForegroundColor(color))?;
    }

    for (idx, ch) in text.chars().enumerate() {
        if let Some(target) = next_match {
            if idx == target {
                queue!(buffer, SetForegroundColor(highlight_fg), Print(ch))?;
                if let Some(color) = default_fg {
                    queue!(buffer, SetForegroundColor(color))?;
                } else if !keep_background {
                    queue!(buffer, ResetColor)?;
                }
                next_match = match_iter.next();
                continue;
            }
        }
        queue!(buffer, Print(ch))?;
    }

    Ok(())
}

pub fn filter_commands(query: &str) -> Vec<&'static CommandEntry> {
    let normalized = command_query_from_input(query);
    COMMANDS
        .iter()
        .filter(|entry| {
            let name = entry.name.to_lowercase();
            let desc = entry.description.to_lowercase();
            fuzzy_match(&normalized, &name) || fuzzy_match(&normalized, &desc)
        })
        .collect()
}

pub fn draw_command_list(
    buffer: &mut Vec<u8>,
    number_of_columns: u16,
    start_row: u16,
    number_of_rows: u16,
    command_line: &str,
    selected_index: usize,
    scroll_offset: usize,
) -> io::Result<()> {
    if number_of_rows == 0 {
        return Ok(());
    }

    let matches = filter_commands(command_line);
    let available_rows = number_of_rows.saturating_sub(3); // blank + header + divider
    let inner_width = number_of_columns.saturating_sub(2); // left/right padding
    let query = command_query_from_input(command_line);
    let name_width = COMMANDS
        .iter()
        .map(|c| c.name.len() as u16)
        .max()
        .unwrap_or(0)
        .min(inner_width);
    let command_col_width = name_width.max(6);
    let desc_col_width = inner_width
        .saturating_sub(command_col_width)
        .saturating_sub(1); // single space between columns

    let mut header = format!(
        "{:<cmd_width$}{}",
        "Command",
        if desc_col_width > 0 {
            format!(" {}", "Description")
        } else {
            String::new()
        },
        cmd_width = command_col_width as usize
    );
    if header.len() > inner_width as usize {
        header.truncate(inner_width as usize);
    } else {
        header.push_str(&" ".repeat(inner_width as usize - header.len()));
    }
    let header_line = format!(" {} ", header);
    queue!(
        buffer,
        MoveTo(0, start_row),
        Clear(ClearType::CurrentLine),
        Print(format!(" {} ", " ".repeat(inner_width as usize)))
    )?;
    queue!(
        buffer,
        MoveTo(0, start_row.saturating_add(1)),
        Clear(ClearType::CurrentLine),
        SetAttribute(Attribute::Bold),
        Print(header_line),
        SetAttribute(Attribute::Reset)
    )?;

    // Divider line under header
    queue!(
        buffer,
        MoveTo(0, start_row.saturating_add(2)),
        Clear(ClearType::CurrentLine),
        Print(format!(" {} ", "─".repeat(inner_width as usize)))
    )?;

    for row in 0..available_rows {
        let screen_row = start_row.saturating_add(row + 3);
        queue!(buffer, MoveTo(0, screen_row), Clear(ClearType::CurrentLine))?;

        if let Some(entry) = matches.get(scroll_offset.saturating_add(row as usize)) {
            let is_selected = selected_index == scroll_offset.saturating_add(row as usize);

            let mut name_display: String = entry
                .name
                .chars()
                .take(command_col_width as usize)
                .collect();
            if name_display.len() < command_col_width as usize {
                name_display.push_str(&" ".repeat(command_col_width as usize - name_display.len()));
            }
            let mut desc_display = String::new();
            if desc_col_width > 0 {
                desc_display = entry
                    .description
                    .chars()
                    .take(desc_col_width as usize)
                    .collect();
                if desc_display.len() < desc_col_width as usize {
                    desc_display
                        .push_str(&" ".repeat(desc_col_width as usize - desc_display.len()));
                }
            }

            let name_matches: Vec<usize> = matched_indices(&query, entry.name)
                .into_iter()
                .filter(|idx| *idx < name_display.chars().count())
                .collect();
            let desc_matches: Vec<usize> = if desc_display.is_empty() {
                Vec::new()
            } else {
                matched_indices(&query, entry.description)
                    .into_iter()
                    .filter(|idx| *idx < desc_display.chars().count())
                    .collect()
            };

            if is_selected {
                queue!(
                    buffer,
                    Print(" "),
                    SetBackgroundColor(Color::DarkGrey),
                    SetForegroundColor(Color::White)
                )?;
                queue_highlighted(
                    buffer,
                    &name_display,
                    &name_matches,
                    Some(Color::White),
                    Color::Yellow,
                    true,
                )?;
                if !desc_display.is_empty() {
                    queue!(buffer, Print(" "))?;
                    queue_highlighted(
                        buffer,
                        &desc_display,
                        &desc_matches,
                        Some(Color::White),
                        Color::Yellow,
                        true,
                    )?;
                }
                queue!(buffer, ResetColor, Print(" "))?;
            } else {
                queue!(buffer, Print(" "))?;
                queue_highlighted(
                    buffer,
                    &name_display,
                    &name_matches,
                    None,
                    Color::Yellow,
                    false,
                )?;
                if !desc_display.is_empty() {
                    queue!(buffer, Print(" "))?;
                    queue_highlighted(
                        buffer,
                        &desc_display,
                        &desc_matches,
                        None,
                        Color::Yellow,
                        false,
                    )?;
                }
                queue!(buffer, ResetColor, Print(" "))?;
            }
        }
    }

    Ok(())
}
