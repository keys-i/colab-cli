# Settings

Settings are local TOML. They live at:

```sh
colab settings path
```

Open the sectioned settings view:

```sh
colab settings
```

In a TTY, `settings`, `settings ui`, and `settings experiments` open the same
interactive editor. It supports multiple edits before one save.

```text
↑/↓ move · enter open/toggle · ←/→ change · space toggle · b/esc back · s save · q quit · ? help
```

`s` writes all pending edits to `config.toml` and stays in the editor. `b` or
Esc returns to the previous screen. Quitting with unsaved changes asks before
discarding. There is no command preview and no advertised search key.

Set one key:

```sh
colab settings set ui.theme auto
colab settings ui set color always
colab settings ui set animations false
colab settings experiments set distribute true
```

Preview terminal colour roles:

```sh
colab settings ui preview
```

Default shape:

```toml
[ui]
color = "auto"
theme = "auto"
animations = true
tui = "auto"
bell = false
fun = false
compact = false
icons = true
unicode = true
neon = true

[output]
json = false
quiet = false
verbose = false
timestamps = false

[debug]
verbose_default = 0
redact_private = true
show_timestamps = false
show_thread_ids = false

[skills]
enabled = true

[experiments]
continue_work = false
distribute = false
multi_login = false
fleet = false
mcp_server = false
ai_plan_runner = false
ast_observer = false
slurp_automation = false
background_live_checks = false

[support]
redact_paths = true
redact_emails = true
redact_tokens = true

[dev]
enabled = false
```

## UI

```sh
colab settings ui
colab settings ui get color
colab settings ui set color auto
colab settings ui set color always
colab settings ui set color never
colab settings ui set neon true
colab settings ui set unicode true
```

`--no-color` is still available as an emergency one-shot override. Normal colour mode belongs in settings.

Interactive UI settings include color mode, neon accents, theme, animations,
terminal bell, fun lines, compact output, icons, unicode, and TUI panels.

Kernel language and selected kernel metadata are cached in the session store,
not in UI settings. Refresh them with:

```sh
colab session kernel refresh
```

## Experiments

Experiments are off by default and saved in `config.toml`.

```sh
colab settings experiments
colab settings experiments get
colab settings experiments get distribute
colab settings experiments set continue true
colab settings experiments set distribute true
colab settings experiments set ast-observer true
colab settings experiments reset
```

Disabled experiments fail with:

```text
experimental feature disabled: distribute
enable: colab settings experiments
```

Experiment gates:

| Experiment | Default | Gates | Note |
|---|---:|---|---|
| Continue | off | `continue` | Checkpoint/replay, not live memory transfer. |
| Distribute | off | `distribute`, hidden `slurp`, hidden `fleet` | Recipes, pools, and shards. No quota bypass. |
| Multi-login | off | distribute profile fallback | Locked unless Distribute is on. |
| MCP server | off | `ai mcp` | Server is still a disabled placeholder unless implemented. |
| AI plan runner | off | `ai plan`, `ai run` | `ai run` also requires `--confirm`. |
| AST observer | off | `run ast`, `run --ast`, `ai code` | Local read-only parser before execution. |
| Background live checks | off | future live status checks | May touch network. |

Private maintainer helpers are hidden under `settings dev` and documented only in [maintainer.md](maintainer.md).
