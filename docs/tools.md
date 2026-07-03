# Tools

The built-in registry lives in `src/cocli/tools/registry.rs`.

Built-ins:

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

`tools run` returns a JSON plan. It does not execute hidden agent actions.

Deferred:

- external plugin loading
- MCP server
- plugin marketplace
