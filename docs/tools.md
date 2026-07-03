# Tools

`cocli-tools` exposes built-in specs:

- `session_new`
- `session_status`
- `exec_python`
- `exec_notebook`
- `fs_list`
- `fs_push`
- `fs_pull`
- `env_install`
- `continue_save`
- `continue_resume`
- `runtime_info`
- `doctor`

Inspect tools:

```sh
colab-cli tools list --json
colab-cli tools inspect fs_push --json
colab-cli tools run fs_push --json '{"src":"./data.csv","dest":"/content/data.csv"}'
```

Tool run output is a JSON plan. The CLI owns credentials, confirmation, and actual remote execution.

MCP:

```sh
cargo build -p colab-cli --features mcp
colab-cli agent mcp --stdio
```

The MCP adapter is experimental and feature-gated.
