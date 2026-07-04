# Command Audit

This pass removed old public command spaces from normal help and kept hidden aliases only where they are cheap.

| Old command/text | New command/text | Location fixed | Alias kept? | Reason |
|---|---|---|---|---|
| `colab-cli exec py` | `colab-cli run py` | clap, README, commands docs, tests | yes | Same action: run Python code. |
| `colab-cli exec run` | `colab-cli run script` | clap, README, commands docs, tests | yes | Script execution belongs with other run/setup work. |
| `colab-cli exec nb` | `colab-cli run notebook` | clap, README, commands docs, tests | yes | Notebook execution is a run command. |
| `colab-cli env install` | `colab-cli run install` | clap, README, commands docs, tests | yes | Package setup prepares a runtime to run code. |
| `colab-cli env freeze` | `colab-cli run freeze` | clap, commands docs, tests | yes | Same runtime setup surface. |
| `colab-cli env restore` | `colab-cli run restore` | clap, commands docs, tests | yes | Same runtime setup surface. |
| `colab-cli mount drive` | `colab-cli fs drive mount` | clap, README, commands docs, tests | yes | Drive is filesystem work. |
| `colab-cli mount list` | `colab-cli fs drive status` | clap, commands docs | yes | Users want Drive state, not a generic mount dump. |
| `colab-cli runtime info` | `colab-cli status runtime` | clap, commands docs, tests | yes | Runtime reads belong under status. |
| `colab-cli runtime info --backend` | `colab-cli status runtime --backend` | clap, README, commands docs, tests | yes | One runtime status surface. |
| `colab-cli runtime gpu` | `colab-cli status runtime --gpu` | clap, commands docs, tests | yes | Same read-only question. |
| `colab-cli runtime tpu` | `colab-cli status runtime --tpu` | clap, commands docs, tests | yes | Same read-only question. |
| `colab-cli runtime versions` | `colab-cli status runtime --versions` | clap, commands docs, tests | yes | Same metadata path. |
| `colab-cli tools list` | `colab-cli ai tools list` | clap, README, commands docs, tests | yes | Agent/tool surfaces belong under the explicit AI command space. |
| `colab-cli tools inspect` | `colab-cli ai tools inspect` | clap, AI docs, tests | yes | Same agent/tool metadata. |
| `colab-cli tools run` | `colab-cli ai run PLAN --confirm` or `settings skills run` for JSON wrappers | clap, AI docs | yes | Execution must be inspectable and confirmation gated. |
| `colab-cli agent ...` | `colab-cli ai ...` | clap, README, commands docs, tests | yes | One public agent-facing command space. |
| `colab-cli config get` | `colab-cli settings get` | clap, commands docs | yes | Local config belongs under settings. |
| `colab-cli config set` | `colab-cli settings set` | clap, README, commands docs | yes | Local config belongs under settings. |
| `colab-cli config path` | `colab-cli settings path` | clap, README, commands docs, tests | yes | Same path lookup. |
| `colab-cli config open` | `colab-cli settings edit` | clap, tests | yes | Editor action reads better as settings edit. |
| `colab-cli doctor quick` | `colab-cli status quick` | clap, README, commands docs, tests | yes | Health checks are status reads. |
| `colab-cli doctor` | `colab-cli status check` | clap, tests | yes | One health-check surface. |
| `colab-cli session status` | `colab-cli status session` | clap, README, commands docs | yes | Session status should not live in two visible places. |
| core command skills | optional agent catalog entries | AI/settings skills docs, tests | n/a | Users run core work as commands; agents inspect thin tool surfaces. |
| snake_case skill names | dotted skill names | AI/settings skills docs, tests | internal alias only | Human output never shows old raw names. |

## Drive Mount Fix

`mount drive` is now only a hidden migration path. It prints:

```text
moved: use `colab-cli fs drive mount`
```

The implementation lives behind `fs drive mount`. It runs `google.colab.drive.mount()` through a Colab kernel cell. It no longer calls the helper from a plain remote `python -c` process, because that process has no IPython kernel and fails with `NoneType`/`kernel` tracebacks.

`agent`, `bug-report`, `server`, `file`, and old alias groups are hidden from top-level help. Public help shows only normal command spaces, `ai`, and completions.
