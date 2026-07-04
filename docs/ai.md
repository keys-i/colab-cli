# AI

`colab ai` is the agent-facing command space. Normal user work stays under `session`, `run`, `fs`, `status`, `auth`, and `settings`. Experimental workflow work is under `distribute` only after it is enabled.

```sh
colab ai tools list
colab ai tools inspect runtime.inspect
colab ai code explain file.py
colab ai code deps file.py
colab ai plan "prepare a local review plan"
colab ai audit plan.toml
colab ai explain plan.toml
colab ai run plan.toml --confirm
```

`ai tools list` is read-only and available by default. It lists ready tool
surfaces such as `runtime.inspect`, `fs.diff`, `fs.changed`, `kernel.list`,
`kernel.restart`, and `ai.audit`.

Recipe/distribute, continue, AST, and MCP rows appear only after their
experiments are enabled.

Package tool rows adapt to cached kernel metadata. Python shows `pkg.python`,
Julia shows `pkg.julia`, and R shows `pkg.r`. The catalog does not list pip as
an agent tool for Julia or R kernels.

AST observation is shown through `run`:

```sh
colab run ast file.py
colab run watch file.py --ast
colab run script file.py --ast --session trainer
```

Execution is gated:

- `ai plan` and `ai run` require `settings experiments set ai-plan-runner true`.
- `ai run` requires `--confirm`.
- A plan file must be supplied explicitly.
- Destructive work must stay inspectable before it runs.
- Tool output must not contain secrets.

MCP serving is gated by `settings experiments set mcp-server true`. If the stdio server is not implemented for a release, the command says so directly instead of pretending to work.

AST/code commands are local and read-only unless the user separately runs code. They never send source to an AI model by themselves.

MCP serving details are covered in [mcp.md](mcp.md). AST details are covered in [ast-observer.md](ast-observer.md).
