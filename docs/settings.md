# Settings

Settings are local TOML. They live at:

```sh
colab-cli settings path
```

Open the sectioned settings view:

```sh
colab-cli settings
```

Set one key:

```sh
colab-cli settings set ui.theme auto
colab-cli settings ui set color always
colab-cli settings ui set animations false
colab-cli settings experiments set distribute true
```

Preview terminal colour roles:

```sh
colab-cli settings ui preview
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
colab-cli settings ui
colab-cli settings ui get color
colab-cli settings ui set color auto
colab-cli settings ui set color always
colab-cli settings ui set color never
colab-cli settings ui set neon true
colab-cli settings ui set unicode true
```

`--no-color` is still available as an emergency one-shot override. Normal colour mode belongs in settings.

## Experiments

Experiments are off by default and saved in `config.toml`.

```sh
colab-cli settings experiments
colab-cli settings experiments get
colab-cli settings experiments get distribute
colab-cli settings experiments set continue true
colab-cli settings experiments set distribute true
colab-cli settings experiments set ast-observer true
colab-cli settings experiments reset
```

Disabled experiments fail with:

```text
experimental feature disabled: distribute
enable: colab-cli settings experiments
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
