# RPC Session

## Purpose
Stdio JSON transport between the UI client and the headless core.

## Key responsibilities
- Spawn core process or connect to it.
- Send requests and receive responses.
- Translate input events into protocol actions.

## Key locations
- Session and transport: `src/rpc_session.rs`, `src/rpc_client.rs`.
