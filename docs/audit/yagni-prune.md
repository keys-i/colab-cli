# YAGNI Prune

This pass keeps migration aliases where cheap, but stops presenting old or speculative surfaces as product features.

| Feature | Removed/merged into | Why |
|---|---|---|
| Top-level `release` | `settings dev release` | Maintainer-only work does not belong in production help. |
| Top-level `doctor` | `status check` | One health-check surface is enough. |
| Top-level `config` | `settings` | Users think in settings, not config plumbing. |
| Top-level `tools` | `ai tools` | Tool wrappers are agent-facing, not a separate product. |
| Top-level `agent` | `ai` | One AI command space is enough. |
| Top-level `runtime` | `status runtime` | Runtime facts are status. |
| Top-level `mount` | `fs drive` | Drive is filesystem work. |
| Top-level `exec` | `run` | Execution belongs under run. |
| Top-level `env` | `run pip` / `run pkg` | Package setup is runtime execution work. |
| `slurp` public surface | `distribute recipe` | Recipe workflows are experimental and opt-in. |
| `fleet` public surface | `distribute pool` | Pool planning is experimental and opt-in. |
| Visible Julia/R command families | `run pkg` | Generic package routing is enough unless active kernel metadata says otherwise. |
| Settings Billing section | `pay` | Billing is a single public action, not a settings pane. |
| Settings Update section | `update` | Update check/install is a single public action, not a settings pane. |
| Disabled AI tool rows | Hidden until enabled | Disabled experiments should not look like available tools. |
| Command preview / Quick Actions | Removed | They were noisy and did not make direct terminal use faster. |
