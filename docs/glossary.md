# Glossary

- Core: headless editor process that owns state and applies commands.
- UI client: terminal UI that renders frames and sends input to the core.
- Frame: snapshot of editor state sent from core to UI for rendering.
- Prompt UI: command/keymap prompt shown in the UI, driven by protocol actions.
