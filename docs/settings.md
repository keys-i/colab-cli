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
colab-cli settings set ui.theme neon
colab-cli settings set ui.interactive false
colab-cli settings set ui.animations false
```

Preview the terminal colour roles:

```sh
colab-cli settings ui preview
```

The important defaults are boring:

```toml
[ui]
theme = "auto"
color = "auto"
interactive = true
animations = true
bell = false
fun = false

[output]
json = false
quiet = false
verbose = false
timestamps = false
```

Owner tools are not part of normal settings. They require the `owner-tools` feature and are documented in [owner-tools.md](owner-tools.md).
