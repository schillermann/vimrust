# VimRust - Terminal-Based Editor

## Motivation

I created this editor to help me learn Rust.
Another reason is to have an editor that does exactly what I want and nothing more.
The goal is to keep the editor as simple as possible without unnecessarily increasing its complexity.

## Quickstart

Start program.
```sh
cargo run
```

Read file (RPC client UI by default).
```sh
cargo run my_file.txt
```

## Keymap

### Normal Mode

|Key |Description |
|----|------------|
|`Q` |Quit        |
|`H` |Cursor Left |
|`J` |Cursor Down |
|`K` |Cursor Up   |
|`L` |Cursor Right|
|`E` |Edit Mode   |
|`S` |Save File   |

### Edit Mode

|Key  |Description|
|-----|-----------|
|`Esc`|Normal Mode|

## Development

### Docs

- [Architecture overview](docs/architecture.md)
- [Core component notes](docs/components/core.md)
- [UI client component notes](docs/components/ui.md)
- [RPC session component notes](docs/components/rpc.md)
- [Protocol component notes](docs/components/protocol.md)
- [Decisions (ADRs)](docs/decisions/README.md)
- [Constraints](docs/constraints.md)
- [Glossary](docs/glossary.md)

### RPC protocol

The default UI spawns a headless core (`vimrust-core`) and speaks JSON over stdio. It does not attach to an existing core process yet.

Headless core (stdio JSON):
```sh
cargo run -p vimrust-core -- file.txt
```

Send line-delimited JSON requests over stdin. All requests:

Core:
- `{"type":"editor_resize","cols":120,"rows":30}`
- `{"type":"editor_resize","cols":120,"rows":30,"suppress_frame":true}` (avoid a frame response)
- `{"type":"state_get"}`
- `{"type":"editor_quit"}`

File:
- `{"type":"file_open","path":"/tmp/foo.txt"}`
- `{"type":"file_save"}`
- `{"type":"file_save_as","path":"/tmp/bar.txt"}`

Text:
- `{"type":"text_insert","text":"hello"}`
- `{"type":"text_delete","kind":"backspace"}`
- `{"type":"text_delete","kind":"under"}`
- `{"type":"line_break"}`

Cursor:
- `{"type":"cursor_move","direction":"left"}`
- `{"type":"cursor_move","direction":"right"}`
- `{"type":"cursor_move","direction":"up"}`
- `{"type":"cursor_move","direction":"down"}`
- `{"type":"cursor_move","direction":"page_up"}`
- `{"type":"cursor_move","direction":"page_down"}`
- `{"type":"cursor_move","direction":"home"}`
- `{"type":"cursor_move","direction":"end"}`

Command UI:
- `{"type":"command_ui","action":"start_prompt"}`
- `{"type":"command_ui","action":"clear"}`
- `{"type":"command_ui","action":"insert_char","ch":"a"}`
- `{"type":"command_ui","action":"backspace"}`
- `{"type":"command_ui","action":"delete"}`
- `{"type":"command_ui","action":"move_left"}`
- `{"type":"command_ui","action":"move_right"}`
- `{"type":"command_ui","action":"move_home"}`
- `{"type":"command_ui","action":"move_end"}`
- `{"type":"command_ui","action":"move_selection_up"}`
- `{"type":"command_ui","action":"move_selection_down"}`
- `{"type":"command_ui","action":"select_from_list"}`

Modes:
- `{"type":"mode_set","mode":"normal"}`
- `{"type":"mode_set","mode":"edit"}`
- `{"type":"mode_set","mode":"prompt_command"}`
- `{"type":"mode_set","mode":"prompt_keymap"}`

Commands:
- `{"type":"command_execute","line":":s"}` (line is optional; without line, the core uses whatever is currently in its command buffer)

Responses include frames with mode, cursor, visible rows, status, file path, and size. Explicit acks confirm operations like open/save/save_as even when no frame is emitted. Errors are sent if a request fails.

### Decisions

#### Editor Async VS Sync

- Vim’s model is a tight, synchronous event loop with occasional timers; it keeps control over redraw timing and avoids shared-state races. That maps best to a single-threaded loop (poll + read), not an async runtime.
- Adding an input thread in a Vim-like editor is only useful if your main thread is blocked on slow I/O; otherwise it introduces concurrency headaches (UI state, buffer edits, redraw ordering) with little gain. If you do heavy background work, it’s usually better to offload that work, not input handling.
- An async EventStream + runtime is a bigger shift from Vim’s architecture. It can integrate timers/network easily, but brings async borrow/lifetime complexity and a dependency/runtime overhead that Vim traditionally avoids.
- Recommendation for a Vim-style editor: stick to a synchronous loop (optionally poll with a short timeout) and keep redraw/input handling on one thread. Spawn threads or async tasks only for background jobs (file IO, LSP, etc.), and deliver their results back to the main loop via a channel.
