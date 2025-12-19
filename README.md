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
1. `{"type":"editor_resize","cols":120,"rows":30}`
2. `{"type":"editor_resize","cols":120,"rows":30,"suppress_frame":true}` (avoid a frame response)
3. `{"type":"file_open","path":"/tmp/foo.txt"}`
4. `{"type":"file_save"}`
5. `{"type":"file_save_as","path":"/tmp/bar.txt"}`
6. `{"type":"text_insert","text":"hello"}`
7. `{"type":"text_delete","kind":"backspace"}`
8. `{"type":"cursor_move","direction":"left"}`
9. `{"type":"command_ui","action":"insert_char","ch":"a"}`
10. `{"type":"mode_set","mode":"command"}`
11. `{"type":"command_execute","line":":s"}` (line is optional; without line, the core uses whatever is currently in its command buffer)
12. `{"type":"state_get"}`
13. `{"type":"editor_quit"}`

Responses include frames with mode, cursor, visible rows, status, file path, and size. Explicit acks confirm operations like open/save/save_as even when no frame is emitted. Errors are sent if a request fails.

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
