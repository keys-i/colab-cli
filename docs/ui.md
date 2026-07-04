# UI

`colab-cli` has two output modes.

Interactive terminals get colour and a small launcher when no command is passed:

```sh
colab-cli
```

Scripts should use explicit commands:

```sh
colab-cli status --json
colab-cli settings skills list --json
```

## Gates

No animation, colour-only decoration, launcher prompts, or fun lines are printed when:

- stdout is not a TTY
- `CI` is set
- `--json` is passed
- `--quiet` is passed
- `--no-color` or `--color never` is passed
- `COLAB_NO_INTERACTIVE=1` is set
- `ui.tui = "never"` is configured

The current launcher is deliberately small. It prints quick actions without command previews. A full-screen TUI can come later if the command flows prove they need it.

## Status

Human status is sectioned text by default:

```text
cocli status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Auth       ✓ ready
Session    ! no active session
Runtime    · pick a session first
```

JSON is only printed with `--json`.
