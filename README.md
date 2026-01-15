# VimrRust

Terminal-based editor with separate core and UI processes.

## Development

### Run

The UI and Core must be started for the editor to function.

Start the core:

```sh
cargo run --bin core
```

Start the UI (in another terminal):

```sh
cargo run --bin ui
```

Call the core directly (no UI):

```sh
curl -v http://127.0.0.1:8080/state
```
