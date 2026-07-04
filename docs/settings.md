# Settings

Settings are local TOML. They live at:

```sh
colab-cli settings path
```

Show the current settings:

```sh
colab-cli settings
```

Set one key:

```sh
colab-cli settings set ui.theme auto
colab-cli settings set ui.animations false
colab-cli settings ui set animations true
colab-cli settings experiments set mcp-server true
```

Preview the terminal colour roles:

```sh
colab-cli settings ui preview
```

The important defaults are boring:

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
multi_login = false
fleet = false
mcp_server = false
ai_plan_runner = false
slurp_automation = false
background_live_checks = false

[support]
redact_paths = true
redact_emails = true
redact_tokens = true

[dev]
enabled = false
```

## Experiments

Experiments are off by default and are saved in `config.toml`.

```sh
colab-cli settings experiments
colab-cli settings experiments get
colab-cli settings experiments set fleet true
colab-cli settings experiments reset
```

Disabled experiments fail with:

```text
experimental feature disabled
enable: colab-cli settings experiments
```

This currently gates multi-login profile workflows, fleet/distributed planning, MCP serving, AI plan running, Slurp automation, and background live checks.

Private maintainer helpers are hidden under `settings dev` and documented only in [maintainer.md](maintainer.md).
