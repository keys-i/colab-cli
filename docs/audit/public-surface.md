# Public Surface Audit

Date: 2026-07-04

## Verified Default Help

`cargo run --bin colab -- --help` shows only:

```text
session
run
fs
status
auth
log
settings
ai
update
version
pay
completions
```

Default help does not show `continue`, `distribute`, `secret`, `release`,
`fleet`, `slurp`, `agent`, `doctor`, `runtime`, `mount`, `exec`, `env`,
`tools`, `config`, or `bug-report`.

## Classification

| Command | Decision | Why | Replacement |
|---|---|---|---|
| `session` | public | Core session lifecycle. | n/a |
| `run` | public | Core execution surface. | n/a |
| `fs` | public | Files and Drive belong together. | n/a |
| `status` | public | Cheap local state and explicit checks. | n/a |
| `auth` | public | Required sign-in/profile flow. | n/a |
| `log` | public | Useful local execution history surface. | n/a |
| `settings` | public | Config, UI, support, experiments. | n/a |
| `ai` | public | Agent-facing read-only tools and audits. | n/a |
| `update` | public | Explicit update command. | n/a |
| `version` | public | Simple version command. | n/a |
| `pay` | public | Opens Colab billing page; no invented status. | n/a |
| `completions` | public | Shell integration. | n/a |
| `continue` | experiment | Checkpoint/replay is optional. | Enable in `settings experiments`. |
| `distribute` | experiment | Recipes/pools/shards are optional and compliant-only. | Enable in `settings experiments`. |
| `secret` | experiment | Secrets bridge is off by default. | Enable in `settings experiments`. |
| `ai mcp` | hidden experiment | MCP server is not a normal stable surface. | Enable MCP experiment; server still reports if not implemented. |
| `ai plan`, `ai run` | hidden experiment | Plan execution is gated and confirmation-only. | Enable AI plan runner. |
| `ai code` | hidden | Code observation belongs under `run ast`. | `colab run ast`. |
| `agent` | hidden disabled alias | Old agent surface bypassed gates. | `colab ai ...`. |
| `fleet` | hidden gated alias | Old pool name. | `colab distribute pool ...`. |
| `slurp` | hidden gated alias | Old recipe name. | `colab distribute recipe ...`. |
| `exec`, `env`, `mount`, `runtime`, `tools`, `config`, `doctor` | hidden aliases | One-cycle migration only. | `run`, `run pip`, `fs drive`, `status runtime`, `ai tools`, `settings`, `status check`. |
| `release` | maintainer-only | Private helper. | `settings dev release` with dev tools and maintainer mode. |
| `server`, `file`, top-level `bug-report` | removed/pruned | Duplicated public surfaces. | `session`, `fs`, `settings support bug-report`. |
| visible `julia` / `r` top-level commands | removed | Kernel language support is internal/adaptive. | `run pkg` or hidden language-specific parser paths. |

## Current Fixes

- Hidden `agent` now stops with a migration error instead of executing a plan.
- `ai --help` shows only stable read-only surfaces: `tools`, `audit`,
  `explain`.
- CLI tests assert default help and `ai --help` do not leak hidden/gated
  surfaces.

## Remaining Rule

Hidden aliases may parse for compatibility, but they must not be documented as
primary commands and must not bypass experiment gates.
