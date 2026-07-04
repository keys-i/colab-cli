# QA

- Top-level help shows only `session`, `run`, `fs`, `status`, `continue`, `slurp`, `fleet`, `ai`, `auth`, `settings`, and `completions`.
- `release`, `agent`, `doctor`, `config`, `tools`, `runtime`, `mount`, `exec`, `env`, and `bug-report` stay out of production help.
- `status` renders a human panel, not raw JSON.
- `status --json`, `settings skills list --json`, and `ai tools list --json` contain no ANSI.
- `settings` renders sectioned settings.
- `settings ui` renders toggles and saves through `settings ui set`.
- `settings experiments` renders all experimental features off by default and saves explicit changes.
- `settings skills list` renders an agent/tool catalog, not core commands.
- `ai tools list` renders the same agent-facing catalog without debug booleans.
- `ai mcp` and `ai run` fail with `experimental feature disabled` until enabled.
- `NO_COLOR=1 colab-cli status` has no ANSI.
- `CI=1 colab-cli status check` does not animate or prompt.
- `session new` keeps the existing prompt flow and renders a concise success card after assignment.
- Transient Colab assignment errors do not dump HTML unless `--verbose` is passed.
- `fs drive mount` shows staged progress in interactive terminals and maps kernel/approval failures to short errors.
- Non-TTY output stays plain unless `--color always` is passed.
- Shipyard remains a release tool first; no broad TUI is required for this pass.
