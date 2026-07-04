# Settings UI Review

Date: 2026-07-04

## Broken Behavior

`colab settings` could render rows scattered diagonally across the terminal.
Controls also felt unreliable because the screen did not redraw as a coherent
vertical menu.

## Root Cause

The settings editor entered raw terminal mode and used normal `println!` line
feeds. In raw mode, `\n` does not reliably imply carriage return, so each row
could start at the previous column instead of column zero.

## Fix

- Settings editor now renders into a bounded string first.
- Raw-mode writes convert `\n` to `\r\n`.
- The renderer clamps width and truncates labels/descriptions.
- The selected row uses a simple vertical marker.
- Footer text only lists implemented controls.

## Width Checks

Unit coverage renders the settings editor at:

```text
60
80
100
140
```

and asserts no rendered line exceeds the target width.

## Current Controls

```text
↑/↓ move · enter open/toggle · ←/→ change · space toggle · b/esc back · s save · q quit
```

These controls are implemented in the settings state machine.

## Decision

Keep the small custom editor for now. Do not add a larger TUI framework unless
the bounded renderer and current key handling fail real terminal QA again.
