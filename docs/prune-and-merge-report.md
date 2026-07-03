# Prune And Merge Report

| Project | Feature A | Feature B | Decision | New command/module | Why |
|---|---|---|---|---|---|
| cocli | `env doctor` | `doctor env` | merge | `doctor env` | diagnostics belong under one command |
| cocli | `mount check` | `doctor mounts` | merge | `doctor mounts` | same user question: what broke? |
| cocli | `runtime backend-info` | `runtime info --backend` | merge | `runtime info --backend` | one runtime info path |
| cocli | `config path` | `config locate` | alias | `config path` | reduces recall without new logic |
| cocli | `fs diff` | `fs changed` | keep alias | `fs changed` | friendlier wording for humans |
| cocli | `fleet cost` | `fleet plan --cost` | merge | `fleet plan --cost` | cost is part of planning |
| cocli | MCP server | agent/tool JSON registry | defer | `agent tools`, `tools list` | no tested standalone MCP server yet |
| Shipyard | `notes` internals | `changelog` internals | merge | `changelog::notes_for` | one grouping path |
| Shipyard | `status` | `doctor` | keep both | `status`, `doctor` | status is state; doctor checks tooling |
| Shipyard | `tag` | `release` | keep standalone | `tag --dry-run` | useful for tag preview |
| Shipyard | `publish` safety | `release` safety | merge internals | `publish::ensure_publish_allowed` | one gate for destructive release work |
| Shipyard | `compare` | `bench` | alias | `bench` | compare wraps bench unless competitor tools exist |
