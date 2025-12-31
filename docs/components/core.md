# Core

## Purpose
Headless editor core that owns editor state and applies commands.

## Key responsibilities
- Maintain buffer, cursor, and mode state.
- Interpret prompt UI actions and command execution.
- Emit frames and acks for the UI client.

## Key locations
- `crates/core/src`.
- Command prompt state: `crates/core/src/prompt_ui_state.rs`.
- Command completion: `crates/core/src/command_completion.rs`.
- Keymap list entries: `crates/core/src/keymap_list.rs`.
