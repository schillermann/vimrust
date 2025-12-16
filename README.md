I created this editor to help me learn Rust.
Another reason is to have an editor that does exactly what I want and nothing more.
The goal is to keep the editor as simple as possible without unnecessarily increasing its complexity.

# Quickstart

Start program.
```sh
cargo run
```

Read file.
```sh
cargo run my_file.txt
```

# RPC protocol

Headless mode (stdio JSON):
```sh
cargo run -- --rpc file.txt
```

Send line-delimited JSON requests over stdin. Examples:
- `{"type":"resize","cols":120,"rows":30}`
- `{"type":"resize","cols":120,"rows":30,"suppress_frame":true}` (avoid a frame response)
- `{"type":"open","path":"/tmp/foo.txt"}`
- `{"type":"insert","text":"hello"}`
- `{"type":"move_cursor","direction":"left"}`
- `{"type":"delete","kind":"backspace"}`
- `{"type":"save_as","path":"/tmp/bar.txt"}`
- `{"type":"get_state"}`
- `{"type":"quit"}`

Responses include frames with mode, cursor, visible rows, status, file path, and size, or errors if a request fails.

# Keymap

# Normal Mode

|Key |Description |
|----|------------|
|`Q` |Quit        |
|`H` |Cursor Left |
|`J` |Cursor Down |
|`K` |Cursor Up   |
|`L` |Cursor Right|
|`E` |Edit Mode   |
|`S` |Save File   |

# Edit Mode

|Key  |Description|
|-----|-----------|
|`Esc`|Normal Mode|

# Decisions

## Editor Async VS Sync

- Vim’s model is a tight, synchronous event loop with occasional timers; it keeps control over redraw timing and avoids shared-state races. That maps best to a single-threaded loop (poll + read), not an async runtime.
- Adding an input thread in a Vim-like editor is only useful if your main thread is blocked on slow I/O; otherwise it introduces concurrency headaches (UI state, buffer edits, redraw ordering) with little gain. If you do heavy background work, it’s usually better to offload that work, not input handling.
- An async EventStream + runtime is a bigger shift from Vim’s architecture. It can integrate timers/network easily, but brings async borrow/lifetime complexity and a dependency/runtime overhead that Vim traditionally avoids.
- Recommendation for a Vim-style editor: stick to a synchronous loop (optionally poll with a short timeout) and keep redraw/input handling on one thread. Spawn threads or async tasks only for background jobs (file IO, LSP, etc.), and deliver their results back to the main loop via a channel.
