# Prune Report

This historical report is superseded by [prune-and-merge-report.md](prune-and-merge-report.md). Current public decisions:

| Feature | Decision | New place | Reason |
|---|---|---|---|
| `env install/freeze/restore` | merge | `run pip install/freeze/restore` | one package surface |
| public `slurp` | merge/gate | `distribute recipe` | one experimental workflow area |
| public `fleet` | merge/gate | `distribute pool` | pool planning is part of distribute |
| top-level `tools` | merge | `ai tools` | one agent-facing command space |
| top-level `agent` | merge | `ai` | same reason |
| top-level `config` | merge | `settings` | one config surface |
| top-level `doctor` | merge | `status check` | diagnostics are status reads |
| top-level `runtime` | merge | `status runtime` | one runtime status path |
| top-level `mount` | merge | `fs drive` | Drive is filesystem work |
| release helpers | hide | `settings dev release` | private maintainer tools |

Internal module names may still be `slurp` and `fleet` until a later rename earns its diff.
