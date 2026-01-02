# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and the project follows
Conventional Commits.

## [Unreleased]

### Added
- Auto completion for file paths to open files in the command line prompt.
- Command prompt history navigation with `Up/Down`.
- Jump between command line and list via `Ctrl-Up/Ctrl-Down`.
- Open the command prompt history file via `:history`.
- Persist command prompt history across sessions in the user state directory.
- Store command history in a `history-commands.txt` file.
- Reload the current file from disk with `:o`/`:open` when no filename is provided.
- Visual mode with selectable ranges.
- Kebab-case transformation command for visual selections via `:case kebab`.
- CamelCase transformation command for visual selections via `:case camel`.
- snake_case transformation command for visual selections via `:case snake`.
- SCREAMING_SNAKE_CASE transformation command for visual selections via `:case screaming`.
- PascalCase transformation command for visual selections via `:case pascal`.
- Train-Case transformation command for visual selections via `:case train`.
- flatcase transformation command for visual selections via `:case flat`.

## [0.1.0] - 2025-12-30

### Added
- Command prompt with a list for discovery and selection.
- Command UI placeholder selection and command execution.
- Keymap prompt with a list for discovery and selection.
- Standard edit functions for insert, delete, backspace, and line breaks.
- Cursor position display in the status line.
- Display mode, file name, and file status in the status line.
