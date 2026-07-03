# Prune Report

| Feature | Current place | Decision | New place | Reason |
|---|---|---|---|---|
| Six publishable crates | `crates/*` | Remove | `src/cocli/*` | No stable public API or external reuse yet. |
| `tools install` | `tools install` | Remove | none | No real plugin format or external plugin exists. |
| `agent mcp` | `agent mcp --stdio` | Remove | none | Not a working tested MCP server. |
| `agent tools` | `agent tools` | Merge | `tools list` | Same registry and same output. |
| Fleet cost command | `fleet cost` | Merge | `fleet plan --cost` | Cost is plan metadata. |
| Fleet local state | `fleet status/stop/clean/topology/logs` | Remove | none | It only wrote local JSON and did not manage runtimes. |
| Fleet compliance | `fleet doctor`, Slurp validation | Keep | `fleet doctor`, `slurp doctor` | Short local checks prevent quota-bypass shapes. |
| Runtime backend info | `runtime backend-info` | Merge | `runtime info --backend` | Same metadata path. Hidden alias remains for compatibility. |
| Env doctor | `env doctor` | Merge | `doctor env` | Diagnostics belong under doctor. |
| Mount check | `mount check` | Merge | `doctor mounts` | Mount checks need a live session; doctor gives the next action. |
| Continuation save/inspect/resume/export/import/clean | `continue` | Keep | `continue` | Honest checkpoint/replay model. |
| `continue resume --dry-run` | missing | Add | `continue resume --dry-run` | Shows replay plan without mutation. |
| `continue last` | missing | Add | `continue last` | Useful latest checkpoint inspection. |
| `session last` | missing | Add | `session last` | Finds the last assigned local session. |
| `--session -` | missing | Add | shared session resolver | Cheap shortcut for last session. |
| `fs changed` | missing | Add | `fs changed` | Alias for local sync planning. |
| `runtime fit --model` | missing | Add | `runtime fit --model MODEL` | Static rough fit check; no exact memory claim. |
| `config open` | missing | Add | `config open` | Uses `$EDITOR`, prints path if unset. |
| `bug-report` | missing | Add | `bug-report` | Redacted local diagnostic JSON. |
| Rare fun lines | requested | Defer | none | Needs UI config plumbing across success paths; not worth touching hot paths now. |
| `fs snack` | requested as optional | Reject | none | Pure joke, no useful operation. |

Useful features added now:

- `doctor quick`
- `doctor paths`
- `doctor --vibe`
- `doctor ferret`
- `session last`
- `--session -`
- `fs changed`
- `slurp doctor`
- `continue last`
- `continue resume --dry-run`
- `runtime fit --model MODEL`
- `config open`
- `bug-report`

Deferred:

- real fleet execution
- live Colab validation for Slurp and runtime fit
- external plugin system
- MCP server
