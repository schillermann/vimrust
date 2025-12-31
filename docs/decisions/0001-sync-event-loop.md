# 0001: Prefer a synchronous event loop

Date: 2025-01-01

## Status
Accepted

## Context
Vim-style editors benefit from a tight, synchronous event loop that retains control over redraw timing and avoids shared-state races. Adding async runtimes or input threads increases complexity and concurrency hazards without clear benefit for this editor's scope.

## Decision
Use a single-threaded, synchronous loop for UI and input handling. Use background threads only for long-running or blocking work, and deliver results back to the main loop via messages.

## Consequences
- Simpler control flow and fewer concurrency concerns.
- Clear separation between UI loop and background tasks.
