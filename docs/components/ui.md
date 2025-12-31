# UI Client

## Purpose
Terminal UI that renders frames and forwards input to the core.

## Key responsibilities
- Render frames into terminal rows.
- Handle input and translate it into protocol requests.
- Maintain lightweight UI-only state.

## Key locations
- Rendering: `src/ui.rs`, `src/ui_editor_rows.rs`, `src/ui_layout.rs`.
- Prompt UI: `src/ui_prompt_line.rs`, `src/ui_prompt_list.rs`.
- Terminal I/O: `src/terminal.rs`.
