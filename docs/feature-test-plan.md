# Feature Test Plan

This file is the checklist for command coverage. Normal CI only runs offline tests. Live Colab tests are opt-in with `COLAB_CLI_LIVE=1`.

## Offline Tests

Run always:

```sh
cargo test --workspace --all-features
```

Coverage:

- command tree parses production spaces
- old aliases stay hidden
- JSON output has no ANSI
- Drive mount uses a kernel cell helper, not plain `python -c`
- Drive kernel-context failures map to a friendly error
- Drive status parsing handles mounted, not mounted, and unknown
- `ai tools list` and `settings skills list` output catalogs, not raw debug rows
- experiments are off by default and block gated commands with a short enable hint

## CLI Parse And Help

Production commands:

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
- `run install`
- `run freeze`
- `run restore`
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
- `status slurp`
- `status fleet`
- `continue save`
- `continue inspect`
- `continue last`
- `continue resume --dry-run`
- `continue export`
- `continue import`
- `continue clean`
- `slurp init`
- `slurp check`
- `slurp plan`
- `slurp explain`
- `slurp run --dry-run`
- `slurp resume --dry-run`
- `fleet plan --dry-run`
- `fleet plan --cost`
- `ai`
- `ai tools list`
- `ai tools list --json`
- `ai tools inspect slurp.plan`
- `ai mcp`
- `ai mcp serve --stdio`
- `ai plan`
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
- `settings skills inspect slurp.plan`
- `settings ui get`
- `settings ui set animations false`
- `completions bash`
- `completions zsh`
- `completions fish`

## Hidden Migration Aliases

Keep for one cycle if they stay cheap:

- `mount drive` -> `fs drive mount`
- `runtime gpu` -> `status runtime --gpu`
- `runtime backend-info` -> `status runtime --backend`
- `tools list` -> `ai tools list`
- `config path` -> `settings path`
- `doctor` -> `status check`
- `exec py` -> `run py`
- `env install` -> `run install`
- `agent` -> `ai`
- `bug-report` -> `settings support bug-report`

Aliases must not appear in normal help.

## Deferred Surfaces

- `fleet status`
- `fleet logs`
- transport MCP server

These stay out of production help or fail behind experiment gates until there is real behaviour behind them.

## Live Smoke

Live tests need real auth and a real session. Run manually:

```sh
COLAB_CLI_LIVE=1 ./scripts/live-smoke.sh
```

Drive mount may require browser approval. In a non-interactive shell the script skips the mount step instead of hanging.
