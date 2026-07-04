# Prune And Merge Report

| Project | Feature A | Feature B | Decision | New command/module | Why |
|---|---|---|---|---|---|
| colab | `env install/freeze/restore` | `run install/freeze/restore` | merge | `run pip install/freeze/restore` | one package-management surface |
| colab | `slurp` | `fleet` | merge | `distribute recipe/pool/shard` | one experimental workflow area |
| colab | `tools` | `agent` | merge | `ai tools`, `ai plan`, `ai audit` | one agent-facing command space |
| colab | `config` | settings | merge | `settings` | local config belongs in one place |
| colab | `doctor` | status | merge | `status check` | diagnostics are status reads |
| colab | `runtime` | status runtime | merge | `status runtime` | one runtime status path |
| colab | `mount` | fs drive | merge | `fs drive` | Drive is filesystem work |
| colab | release helpers | public CLI | hide | `settings dev release` | private maintainer tools only |
| colab | MCP server | tool catalog | defer server | `ai mcp` gated | no fake server; return honest not-implemented error |
| Shipyard | release UI polish | broader TUI | keep sober | existing release surfaces | release tool first |

Internal module names `slurp` and `fleet` remain for the existing parser/planner implementation. User-facing docs should say `recipe`, `pool`, `shard`, and `distribute`.
