# QA

- Top-level help shows only `session`, `run`, `fs`, `status`, `continue`, `slurp`, `fleet`, `auth`, `settings`, and `completions`.
- `release`, `agent`, `doctor`, `config`, `tools`, `runtime`, `mount`, `exec`, `env`, and `bug-report` stay out of production help.
- `status` renders a human panel, not raw JSON.
- `status --json` and `settings skills list --json` contain no ANSI.
- `settings` renders sectioned settings.
- `settings ui` renders toggles and saves through `settings ui set`.
- `settings skills list` renders an agent/tool catalog, not core commands.
- `NO_COLOR=1 colab-cli status` has no ANSI.
- `CI=1 colab-cli status check` does not animate or prompt.
- `session new` keeps the existing prompt flow and renders next actions after assignment.
- Non-TTY output stays plain unless `--color always` is passed.
- Shipyard remains a release tool first; no broad TUI is required for this pass.
