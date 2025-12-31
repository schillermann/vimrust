# Constraints

## Design
- Keep the editor simple and avoid unnecessary complexity.
- Prefer a single-threaded, synchronous event loop for UI and input.

## Protocol
- UI and core communicate over JSON stdio.
- Responses include frames and explicit acks for key operations.

## Code style
- Follow `AGENTS.md` rules for object-oriented design and method naming.
