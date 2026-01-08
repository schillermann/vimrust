# 0002 Core renders rows

## Status
Accepted

## Context
The system uses a headless core and a thin UI client over JSON stdio.
Frames currently carry visible rows rather than raw file lines.
We want consistent display rules and a small, stable UI and plugin surface.

Neovim uses a similar model: the core renders into a grid, and UIs paint it.

## Decision
Keep rendering of visible rows in the core.
The UI renders frames to the terminal without reimplementing editor display rules.

## Rationale
- Consistent display across plugins and UIs (tabs, control chars, folds, highlights).
- Thinner UI clients that only paint rows.
- Efficient updates via row deltas instead of raw lines plus rules.
- Plugin effects are composed in one place, then rendered.
- Stable plugin API: plugins consume a ready-to-render view without reimplementing tab expansion, control-char rules, or viewport math.
- Smaller plugin surface area: no need to expose raw buffers or presentation rules.

## Consequences
- UI has less flexibility for custom rendering without core support.
- Changing presentation rules requires core changes and protocol updates.
