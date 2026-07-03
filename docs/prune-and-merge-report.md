# Prune And Merge Report

| Project | Feature A | Feature B | Decision | New command/module | Why |
|---|---|---|---|---|---|
| cocli | `env doctor` | `doctor env` | merge | `status run` | diagnostics belong under one status surface |
| cocli | `mount check` | `doctor mounts` | merge | `status drive` | same user question: what broke? |
| cocli | `runtime backend-info` | `runtime info --backend` | merge | `status runtime --backend` | one runtime status path |
| cocli | `config path` | `config locate` | alias | `settings path` | reduces recall without new logic |
| cocli | `fs diff` | `fs changed` | keep alias | `fs changed` | friendlier wording for humans |
| cocli | `fleet cost` | `fleet plan --cost` | merge | `fleet plan --cost` | cost is part of planning |
| cocli | MCP server | skills JSON registry | defer | `settings skills list` | no tested standalone MCP server yet |
| Shipyard | `notes` internals | `changelog` internals | merge | `changelog::notes_for` | one grouping path |
| Shipyard | `status` | `doctor` | keep both | `status`, `doctor` | status is state; doctor checks tooling |
| Shipyard | `tag` | `release` | keep standalone | `tag --dry-run` | useful for tag preview |
| Shipyard | `publish` safety | `release` safety | merge internals | `publish::ensure_publish_allowed` | one gate for destructive release work |
| Shipyard | `compare` | `bench` | alias | `bench` | compare wraps bench unless competitor tools exist |
