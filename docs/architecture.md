# Architecture

## Goals
- Keep the editor simple and focused.
- Prefer a single-threaded, synchronous loop for UI and input handling.
- Use a headless core with a thin UI client over JSON stdio.

## High-level structure
- crates/core: headless editor core (buffer, modes, prompt UI, command execution).
- crates/protocol: JSON protocol types for core <-> UI communication.
- src: UI client and runtime glue (terminal, rendering, RPC session).

## Entry points
- UI client: `src/main.rs` (spawns or connects to the core and renders frames).
- Headless core: `crates/core` binary (stdio JSON protocol).

## Data flow (simplified)
- UI sends JSON requests to core.
- Core replies with frames and acks.
- UI renders frames to terminal and updates local UI state.

## Where to look first
- Protocol definitions: `crates/protocol/src`.
- Core behavior: `crates/core/src`.
- UI and RPC session: `src/rpc_session.rs`, `src/ui.rs`.

## Protocol module map
```
crates/protocol/src
├─ lib.rs (re-exports)
├─ frame.rs
│  ├─ uses: prompt_ui.rs (CommandUiFrame)
│  ├─ uses: status.rs (StatusMessage)
│  ├─ uses: path.rs (FilePath)
│  └─ uses: version.rs (ProtocolVersion)
├─ rpc.rs
│  ├─ uses: prompt_ui.rs (CommandUiAction)
│  ├─ uses: frame.rs (Frame)
│  ├─ uses: status.rs (StatusMessage)
│  └─ uses: path.rs (FilePath)
├─ prompt_ui.rs (CommandUiFrame, CommandUiAction, PromptInputSelection, PromptMode)
├─ status.rs (StatusMessage)
├─ path.rs (FilePath)
└─ version.rs (ProtocolVersion)
```
