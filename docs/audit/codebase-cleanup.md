# Codebase Cleanup Audit

Scope: static repo pass on July 4, 2026. No code was changed. The repo had untracked `colabtools/` and `google-colab-cli/` trees; this audit treats them as imported reference material and leaves them alone.

## Findings

| Area | Evidence | Risk | Cleanup |
|---|---|---|---|
| Package and binary names | Package/crate remains `colab-cli` / `colab_cli`; primary binary is `colab`; compatibility binary is `colab-cli`. | Low external risk; users run `colab` while Rust imports keep the crate name. | Keep this split documented. |
| Cargo description | `Cargo.toml` says "sessions, code execution, files, Drive, status, auth, logs, and agent-facing tools." | No old `exec` wording remains in metadata. | clear |
| Imported Python CLI is untracked | `google-colab-cli/` contains public `colab new`, `colab exec`, `colab install`, `colab update`, etc. | Search results and audits are noisy; easy to mistake old Python docs for current product docs. | Keep excluded from public release/package paths. Consider moving under `references/` if it stays. |
| Imported colabtools tree is untracked | `colabtools/` contains `google.colab.*` implementation and tests. | Same noise risk; may look like owned public surface. | Keep as reference-only or delete when feature mapping is done. |
| `slurp`/`fleet` internal names remain | `src/cocli/slurp`, `src/cocli/fleet`, `SlurpConfig`, `FleetCommands`, `fleet_name`. | Internal names bleed into config schemas, tests, and error strings. | Do not rename now. Rename only if `distribute` graduates from experimental. |
| `slurp.toml` fallback remains | `recipe_config()` falls back from `colab.recipe.toml` to `slurp.toml`. | Intentional migration support, but keeps old name alive. | Keep while hidden `slurp` alias exists; remove together. |
| Duplicate experiment keys | Config has `distribute`, `fleet`, and `slurp_automation`; `settings experiments set fleet` and `set slurp_automation` both turn on distribute. | Users can persist stale keys that no visible UI explains. | Hide from docs; prune stale keys after one migration cycle. |
| Command-audit docs overstate `release` alias | Docs list top-level `release` as hidden, but tests assert `colab release ...` does not parse. Maintainer release is `settings dev release` behind features/env. | Maintainers may test the wrong command. | Update existing audit docs later; this audit records `release` as maintainer-only, not a hidden public alias. |
| `tools` compatibility comment disagrees with dispatch | `args.rs` says old `tools` moved to `settings skills`; dispatch migrates to `colab ai tools ...`. | Maintainers may preserve the wrong alias target. | Treat `ai tools` as public tool catalog; `settings skills` is local enable/disable/admin. |
| Visible `auth` diagnostics overlap | `auth doctor`, `auth export-redacted`, `auth limits` are visible in `auth --help`; diagnostics/support already exist under `status check` and `settings support`. | Public command zoo grows inside a kept top-level space. | Move or hide after deciding whether auth profile management is user-facing or maintainer support. |
| Dynamic `run --help` hides implemented subcommands | `RunCommands` includes `pip`, `julia`, `r`, `ast`, `watch`, `last`, `history`; default help showed only `code`, `script`, `notebook`, `repl`, `shell`, `pkg`. | Docs and live help can disagree. This is intentional for kernel-aware help, but audit scripts must check both parse and help. | Keep dynamic help, but add a surface test for explicit hidden/visible status. |
| Stale top-level compatibility groups still parse | `exec`, `env`, `mount`, `runtime`, `tools`, `config`, `doctor`, `agent`, `server`, `file`, `new`, `sessions`, `stop`, `upload`, `download`, `log`. | Good migration path, but every alias is another behavior to preserve. | Keep only the migration aliases with real users; prune `server`/`file` first unless there is a documented migration need. |
| Experimental surfaces are real but off | `continue`, `distribute`, `ai mcp`, `ai plan`, `ai run`, `run ast`/`watch` are gated by config. | If docs show examples without gate text, users see disabled commands. | Keep gated. Public docs should mark them experimental every time. |
| Tool registry has legacy snake names | `src/cocli/tools/registry.rs` maps dotted names plus `legacy_name()` such as `slurp_plan`, `fleet_plan`. | Agent/tool consumers may learn the wrong identifiers. | Keep legacy lookup only; never print snake names in human output. |
| Test coverage preserves aliases | `tests/cli.rs::hidden_aliases_parse_for_one_cycle` asserts old aliases parse. | The "one cycle" never ends unless tracked. | Add a removal issue/date before release 1.0. |
| Surface check script is narrow | `scripts/check-command-surface.sh` checks top-level help and old examples, but not visible nested auth/settings/ai surfaces. | Command leaks can hide below top level. | Extend later to scrape `auth --help`, `ai --help`, `settings --help`, and gated help. |

## Repeated Feature Clusters

| Cluster | Current kept surface | Repeated/old surface | Decision |
|---|---|---|---|
| Python execution | `run py`, `run code`, `run script`, `run notebook`, `run repl`, `run shell` | `exec py/run/nb/repl/shell`, Python `colab exec`, `server run` | Keep `run`; hide `exec`; consider pruning `server run`. |
| Package setup | `run pkg`, `run pip`, `run julia pkg`, `run r pkg`, `run r renv` | `env install/freeze/restore`, `run install/freeze/restore`, Python `colab install` | Keep `run pkg`/language package commands; hidden aliases can expire. |
| Runtime status | `status runtime` | `runtime info/gpu/tpu/versions/backend-info` | Keep `status runtime`; hidden `runtime` can expire. |
| Drive/mount | `fs drive mount/status/list/unmount/path` | `mount drive/list`, Python `colab drivemount` | Keep `fs drive`; hidden `mount` can expire. |
| Files | `fs ls/push/pull/rm/edit/sync/diff/changed` | `file upload/download/ls/cp/rm`, Python `colab upload/download/ls` | Keep `fs`; prune old Rust `file` unless compatibility demand exists. |
| Diagnostics | `status quick/check/paths/version`, `settings support` | `doctor`, `auth doctor`, `bug-report` | Keep `status`/`settings support`; decide whether `auth doctor` earns visibility. |
| Agent/tools | `ai tools`, `ai code`, `settings skills` | `tools`, `agent`, snake tool names | Keep `ai` for use, `settings skills` for admin; hide old names. |
| Workflow distribution | `distribute recipe/pool/shard` gated | `slurp`, `fleet`, `slurp.toml`, `SlurpConfig`, `FleetCommands` | Keep gated `distribute`; internal names can wait. |
| Continuation | `continue` gated | manifest fields like `fleet_name` | Keep gated; clean names if feature graduates. |
| Maintainer release | `settings dev release` with feature/env guard | old docs mention top-level `release` | Keep maintainer-only. No top-level alias. |

## Stale Docs And Tests

| File | Stale point | Action |
|---|---|---|
| `Cargo.toml` | Package is `colab-cli`, primary binary is `colab`. | Keep install docs explicit: `cargo install colab-cli`, run `colab`. |
| `docs/command-audit.md` | Lists `release` among hidden top-level commands; code rejects top-level `release` unless future feature adds it. | Clarify as maintainer-only. |
| `docs/qa.md` | Same hidden-list wording includes `release`. | Clarify no top-level release alias. |
| `docs/settings.md` | Shows `fleet = false` and `slurp_automation = false` config keys. | Mark as compatibility-only or remove from user docs. |
| `tests/cli.rs` | "one cycle" alias test has no expiry. | Add issue/date or split into aliases-to-keep vs aliases-to-prune. |
| `scripts/check-command-surface.sh` | Does not check nested visible leaks like `auth doctor`. | Extend after deciding desired nested surface. |

## Prune Order

1. Prune or hide old Rust `server` and `file` groups first. They are hidden, not mentioned as Python migration aliases, and duplicate `session`/`fs`.
2. Prune top-level `bug-report`; keep `settings support bug-report`.
3. Expire `env`, `exec`, `mount`, `runtime`, `tools`, `config`, `doctor`, `log`, `new`, `sessions`, `stop`, `upload`, `download` only after one documented migration window.
4. Keep `slurp`/`fleet` hidden until `distribute` either graduates or is removed.
5. Leave internal module renames (`slurp`, `fleet`) for a mechanical cleanup after public surface decisions settle.
