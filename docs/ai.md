# AI

`colab-cli ai` is the agent-facing command space. Normal user work stays under `session`, `run`, `fs`, `status`, `auth`, and `settings`. Experimental workflow work is under `distribute` only after it is enabled.

```sh
colab-cli ai tools list
colab-cli ai tools inspect recipe.plan
colab-cli ai ast file.py
colab-cli ai ast watch file.py
colab-cli ai code explain file.py
colab-cli ai code deps file.py
colab-cli ai plan "prepare a local review plan"
colab-cli ai audit plan.toml
colab-cli ai explain plan.toml
colab-cli ai run plan.toml --confirm
```

`ai tools list` is read-only and available by default. It lists optional tool surfaces such as `recipe.plan`, `distribute.plan`, `fs.diff`, `ast.outline`, `mcp.tools`, and `ai.audit`.

Execution is gated:

- `ai plan` and `ai run` require `settings experiments set ai-plan-runner true`.
- `ai run` requires `--confirm`.
- A plan file must be supplied explicitly.
- Destructive work must stay inspectable before it runs.
- Tool output must not contain secrets.

MCP serving is gated by `settings experiments set mcp-server true`. If the stdio server is not implemented for a release, the command says so directly instead of pretending to work.

AST/code commands are local and read-only unless the user separately runs code. They never send source to an AI model by themselves.

MCP serving details are covered in [mcp.md](mcp.md). AST details are covered in [ast-observer.md](ast-observer.md).
