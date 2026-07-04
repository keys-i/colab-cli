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

[support]
redact_paths = true
redact_emails = true
redact_tokens = true

[dev]
enabled = false
```

Private maintainer helpers are hidden under `settings dev` and documented only in [maintainer.md](maintainer.md).
