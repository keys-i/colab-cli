# UI Review

Scope: current terminal rendering, settings screen, UI docs, and UI-related tests.

## Findings

- The visible top-level surface is tight: `session`, `run`, `fs`, `status`, `ai`, `auth`, `settings`, and `completions`. Hidden legacy surfaces stay out of help.
- Human output is sectioned text, not a broad TUI. `status` renders a compact panel and `settings` renders a sectioned menu in non-TTY mode.
- JSON mode is separate and covered by tests for no ANSI output.
- Settings are split into direct commands and an interactive editor. The editor batches edits before save and has tests for state navigation and the multi-login lock.
- Table rendering has local width tests and truncates long cells, which is enough for the current catalog/status tables.
- `Ui::new(quiet, plain, interactive)` is intentionally small. No UI framework is needed for this CLI.

## Risks

- `settings` currently reads the real config path in render probes; tests isolate with temp `HOME`, but manual probes can reflect maintainer-local state.
- The interactive settings editor is tested at state level, not by terminal-key integration. That is acceptable for now, but raw-mode regressions need manual smoke testing.
- Some rendered docs use ideal defaults while local config can differ, such as `bell`.

## Test Plan

- `cargo test --all-targets`
- `target/debug/colab --help`
- `target/debug/colab status`
- `target/debug/colab settings`
- `target/debug/colab --json ai tools list`
- Manual TTY smoke before release: open `settings`, toggle one UI value, save, quit, then confirm raw mode is restored.

## YAGNI

- Skip a full-screen TUI until repeated workflows need persistent panes.
- Skip snapshot testing every ANSI string; keep stable JSON and a few human-output assertions.
- Skip theme expansion beyond current color roles. Add only when a real terminal/accessibility issue appears.
