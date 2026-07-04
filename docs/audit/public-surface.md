# Public Surface Audit

Scope: current Rust CLI as observed from `args.rs`, `dispatch.rs`, tests, docs, and live `cargo run -- --help` on July 4, 2026. No code was changed.

## Public Contract

Default top-level help should stay compact:

```text
session, run, fs, status, auth, log, settings, ai, update, version, pay, completions
```

Everything else should be one of: hidden migration alias, gated experiment, maintainer-only, or pruned.

## Command Disposition

| Command surface | Disposition | Public replacement / owner | Notes |
|---|---|---|---|
| `session` | Keep public | `session` | Core lifecycle. |
| `session new/list/stop/url/last/refresh/repair/reconnect/logs/kernel` | Keep public | `session ...` | `session status` is hidden; use `status session`. |
| `session list` alias `ls` | Hide as alias | `session list` | Cheap shell habit. Do not document as primary. |
| `run` | Keep public | `run` | Core execution/setup. |
| `run code/py/script/notebook/repl/shell/pkg` | Keep public | `run ...` | Dynamic help may hide Python-specific package tools depending on kernel metadata. |
| `run script` alias `run` | Hide as alias | `run script` | Avoid `colab run run`. |
| `run notebook` alias `nb` | Hide as alias | `run notebook` | Migration convenience only. |
| `run pip` | Keep public for Python kernels | `run pip`, `run pkg` | Python-specific package helper; hidden from dynamic help for non-Python kernels. |
| `run julia`, `run r` | Hide as parser paths | `run pkg` | Do not advertise language-specific families unless active kernel metadata justifies it. |
| `run ast`, `run watch` | Keep experimental | `run ast`, `run watch` | Gate with AST observer messaging/docs. |
| `run install/freeze/restore` | Hide as aliases | `run pip install/freeze/restore` | One-cycle migration. |
| `run last/history` | Keep public, watch | `run last`, `run history` | Local history is useful but should stay local-only and explicit. |
| `fs` | Keep public | `fs` | Core file/Drive area. |
| `fs ls/upload/download/rm/edit/sync/diff/changed` | Keep public | `fs ...` | `push/pull` are hidden aliases. `fs sync` is planning/dry-run in this release. |
| `fs drive mount/status/list/unmount/path` | Keep public | `fs drive ...` | Drive belongs under filesystem. |
| `status` | Keep public | `status` | Default should remain cheap local status. |
| `status session/runtime/auth/fs/drive/kernel/quick/check/run/paths/version` | Keep public | `status ...` | Read-only and diagnostic. |
| `status slurp`, `status fleet` | Hide as aliases | `distribute status` / `distribute pool status` | Old workflow names. |
| `ai` | Keep public | `ai` | Agent-facing user surface. |
| `ai tools list/inspect` | Keep public | `ai tools ...` | Read-only tool catalog. |
| `ai code explain/deps` | Keep public | `ai code ...` | Local code inspection. |
| `ai mcp` | Keep experimental | `ai mcp` | Gated; `serve` currently reports not implemented. |
| `ai plan/audit/explain/run` | Keep experimental | `ai ...` | Plans must be explicit; execution needs confirmation/gate. |
| `ai ast` | Hide as alias | `run ast` / `run watch` | AST is a run aid, not primary AI surface. |
| `auth login/logout/add/list/status/use/remove` | Keep public | `auth ...` | Profile/account management. |
| `auth doctor` | Hide or move | `status auth` / `status check` | Visible now; overlaps diagnostics. |
| `auth export-redacted` | Hide or move | `settings support bug-report` / `settings support redact` | Visible now; support belongs under settings. |
| `auth limits` | Hide or keep experimental | `settings experiments` / profile policy docs | Visible now; tied to multi-login/distribute policy. |
| `settings` | Keep public | `settings` | Local config/admin. |
| `settings get/set/path/edit/reset/about` | Keep public | `settings ...` | `path` alias `locate` should remain hidden. |
| `settings ui` | Keep public | `settings ui ...` | User preference surface. |
| `settings experiments` | Keep public | `settings experiments ...` | Gate control must be discoverable. |
| `settings skills` | Keep public/admin | `settings skills ...` | Enable/disable/run tool specs; `ai tools` is the user catalog. |
| `settings support` | Keep public | `settings support ...` | Diagnostics/support bundle/redaction. |
| `settings update`, `settings billing` | Hide as compatibility/admin paths | `update`, `pay` | Top-level names match the installed `colab` reference CLI. |
| `settings dev release` | Maintainer-only | feature/env gated | Requires `dev-tools`/`owner-tools` and maintainer switch. |
| `completions` | Keep public | `completions` | Shell integration. |
| `log` | Keep public | `log` | Familiar history/export entry point; no fake remote logs. |
| `update` | Keep public | `update` | Explicit update check/install surface. |
| `version` | Keep public | `version` | Simple version output. |
| `pay` | Keep public | `pay` | Opens Colab billing; no invented billing state. |
| `continue` | Keep experimental | `continue ...` | Hidden top-level; gated by `continue`. Does not restore live memory. |
| `distribute` | Keep experimental | `distribute recipe/pool/shard` | Hidden top-level; gated by `distribute`. No quota bypass. |
| `slurp` | Hide as alias | `distribute recipe` | Old name; keep only while migration is active. |
| `fleet` | Hide as alias | `distribute pool` | Old name; keep only while migration is active. |
| `exec` | Hide as alias | `run` | Old execution group. |
| `env` | Hide as alias | `run pip` | Old package group. |
| `mount` | Hide as alias | `fs drive` | Old Drive group. |
| `runtime` | Hide as alias | `status runtime` | Old runtime status group. |
| `tools` | Hide as alias | `ai tools` | Comment in `args.rs` says `settings skills`; dispatch says `ai tools`. Use `ai tools`. |
| `config` | Hide as alias | `settings` | Old config group. |
| `doctor` | Hide as alias | `status check` | Old diagnostic group. |
| `agent` | Hide as alias | `ai` | Old agent group. |
| `server` | Prune | `session`, `run shell`, `status`, `fs` | Old Rust group, not a Python migration alias. |
| `file` | Prune | `fs` | Old Rust group, duplicates `fs`. |
| `new`, `sessions`, `stop`, top-level `upload`, top-level `download` | Hide as aliases | `session new/list/stop`, `fs upload/download` | Python `colab` migration aliases. |
| `bug-report` | Prune top-level | `settings support bug-report` | Hidden top-level support shortcut is not needed. |
| `release` | Maintainer-only, not alias | `settings dev release` | Current tests assert top-level `release` does not parse. |
| `drivemount`, top-level `install`, `restart-kernel` | Prune / do not add | `fs drive mount`, `run pip install`, `session kernel restart` | Mention only in migration docs. |

## Command Zoo Risks

| Risk | Why it matters | Guardrail |
|---|---|---|
| Hidden aliases become permanent | Tests assert "one cycle" aliases parse but no expiry exists. | Track aliases with a removal milestone. |
| Nested visible leaks | Top-level help is clean, but `auth --help` exposes support/diagnostic commands. | Audit nested help, not just top level. |
| Dynamic help hides real commands | `run --help` adapts to cached kernel language and can omit commands that parse. | Surface tests should parse the full enum and scrape representative help states. |
| Imported reference trees pollute search | `google-colab-cli/` and `colabtools/` contain many old public commands. | Keep release/package include lists tight; move references if needed. |
| Old internal names leak through tools | `legacy_name()` accepts snake names like `slurp_plan` and `fleet_plan`. | Accept old input, print dotted names only. |
| Gated experiments look public in docs | `continue` and `distribute` have docs and examples but fail until enabled. | Every example block should include gate setup or "disabled by default". |
| Maintainer commands look user-facing | `settings dev release` exists only in feature/env builds. | Keep in `docs/maintainer.md`, not user quick starts. |

## Keep/Hide/Prune Summary

| Disposition | Surfaces |
|---|---|
| Keep public | `session`, `run`, `fs`, `status`, `auth`, `log`, `settings`, `ai`, `update`, `version`, `pay`, `completions` and their core subcommands listed above. |
| Hide as aliases | `exec`, `env`, `mount`, `runtime`, `tools`, `config`, `doctor`, `agent`, `slurp`, `fleet`, `new`, `sessions`, `stop`, `upload`, `download`, `log`, `session ls`, `run nb`, `run install/freeze/restore`. |
| Keep experimental | `continue`, `distribute`, `run ast/watch`, `ai mcp`, `ai plan/audit/explain/run`, multi-login/distribute controls. |
| Maintainer-only | `settings dev release`; no top-level `release`. |
| Prune | `server`, `file`, top-level `bug-report`, old flat commands not implemented as aliases (`pay`, `version`, `update`, `drivemount`, top-level `install`, `restart-kernel`). |
