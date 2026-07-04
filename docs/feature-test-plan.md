# Feature Test Plan

Normal CI only runs offline tests. Live Colab tests are opt-in with `COLAB_CLI_LIVE=1`.

## Offline Tests

Run always:

```sh
cargo test --workspace --all-features
```

Coverage:

- default command tree parses production spaces
- optional spaces stay hidden until enabled
- old aliases stay hidden
- JSON output has no ANSI
- Drive mount uses a kernel cell helper, not plain `python -c`
- Drive kernel-context failures map to a friendly error
- Drive status parsing handles mounted, not mounted, and unknown
- `ai tools list` and `settings skills list` output catalogs, not raw debug rows
- experiments are off by default and block gated commands with a short enable hint

## CLI Parse And Help

Default production commands:

- `session new`
- `session list`
- `session last`
- `session stop`
- `session url`
- `run py`
- `run script`
- `run notebook`
- `run shell`
- `run repl`
- `run pip install`
- `run pip freeze`
- `run pip restore`
- `run last`
- `run history`
- `fs ls`
- `fs push`
- `fs pull`
- `fs rm`
- `fs edit`
- `fs sync --dry-run`
- `fs changed`
- `fs diff`
- `fs drive mount`
- `fs drive status`
- `fs drive list`
- `fs drive unmount`
- `fs drive path`
- `status`
- `status quick`
- `status check`
- `status session`
- `status auth`
- `status runtime --all`
- `status runtime --gpu`
- `status runtime --tpu`
- `status runtime --backend`
- `status runtime --versions`
- `status fs`
- `status drive`
- `ai`
- `ai tools list`
- `ai tools list --json`
- `ai tools inspect recipe.plan`
- `run ast`, gated
- `ai mcp`, gated
- `ai plan`, gated
- `ai audit`
- `auth list`
- `auth status`
- `auth export-redacted`
- `settings get`
- `settings set`
- `settings path`
- `settings edit`
- `settings reset`
- `settings experiments`
- `settings experiments get`
- `settings experiments set mcp-server true`
- `settings experiments reset`
- `settings skills list`
- `settings skills list --json`
- `settings skills inspect recipe.plan`
- `settings ui get`
- `settings ui set animations false`
- `completions bash`
- `completions zsh`
- `completions fish`

Experimental commands:

- `continue save`
- `continue inspect`
- `continue last`
- `continue resume --dry-run`
- `continue resume --confirm`
- `continue export`
- `continue import`
- `continue clean`
- `distribute plan`
- `distribute status`
- `distribute recipe init`
- `distribute recipe explain`
- `distribute pool plan --cost`
- `distribute shard plan`

## Hidden Migration Aliases

Keep for one cycle if they stay cheap:

- `mount drive` -> `fs drive mount`
- `runtime gpu` -> `status runtime --gpu`
- `runtime backend-info` -> `status runtime --backend`
- `tools list` -> `ai tools list`
- `config path` -> `settings path`
- `doctor` -> `status check`
- `exec py` -> `run py`
- `env install` -> `run pip install`
- `run install` -> `run pip install`
- `slurp` -> `distribute recipe`
- `fleet` -> `distribute pool`
- `agent` -> `ai`
- `bug-report` -> `settings support bug-report`

Aliases must not appear in normal help.

## Deferred Surfaces

- dynamic help that reveals enabled experiments
- transport MCP server
- exact Tree-sitter AST

These stay hidden, gated, or honest about not being implemented until there is real behaviour behind them.

## Live Smoke

Live tests need real auth and a real session. Run manually:

```sh
COLAB_CLI_LIVE=1 ./scripts/live-smoke.sh
```

Drive mount may require browser approval. In a non-interactive shell the script skips the mount step instead of hanging.
