# UI

`colab` has human output and JSON output.

Interactive terminals get colour and simple prompts only where selection helps:

```sh
colab settings
colab session
```

Scripts should use explicit commands:

```sh
colab status --json
colab ai tools list --json
```

## Gates

No animation, colour-only decoration, launcher prompts, or fun lines are printed when:

- stdout is not a TTY
- `CI` is set
- `--json` is passed
- `--quiet` is passed
- `--no-color` is passed
- `ui.color = "never"` is configured
- `COLAB_NO_INTERACTIVE=1` is set
- `ui.tui = "never"` is configured

Running `colab` with no args prints help. It does not print Quick Actions or command previews.

## Status

Human status is sectioned text by default:

```text
cocli status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Auth       ✓ ready
Session    ! no active session
Runtime    · pick a session first
Files      ✓ cache writable
Drive      · not checked

fix: run colab session list
```

JSON is only printed with `--json`.
