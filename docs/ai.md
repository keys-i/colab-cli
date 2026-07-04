# AI

`colab-cli ai` is the agent-facing command space. Normal user work stays under `session`, `run`, `fs`, `status`, `continue`, `slurp`, and `fleet`.

```sh
colab-cli ai tools list
colab-cli ai tools inspect slurp.plan
colab-cli ai plan "prepare a local review plan"
colab-cli ai audit plan.toml
colab-cli ai explain plan.toml
colab-cli ai run plan.toml --confirm
```

`ai tools list` is read-only and available by default. It lists optional tool surfaces such as `slurp.plan`, `fleet.plan`, `fs.diff`, `continue.resume`, `mcp.tools`, and `ai.audit`.

Execution is deliberately gated:

- `ai run` requires `settings experiments set ai-plan-runner true`.
- `ai run` requires `--confirm`.
- A plan file must be supplied explicitly.
- Destructive work must stay inspectable before it runs.
- Tool output must not contain secrets.

MCP serving is covered in [mcp.md](mcp.md).
