# Command Audit

This pass keeps normal commands visible and hides optional/experimental or maintainer-only command spaces.

Default public help shows:

```text
session, run, fs, status, ai, auth, settings, completions
```

Hidden unless enabled or kept as compatibility aliases:

```text
continue, distribute, slurp, fleet, release, agent, doctor, config, tools, runtime, mount, exec, env, bug-report
```

| Old command/text | New command/text | Alias kept? | Reason |
|---|---|---:|---|
| `colab exec py` | `colab run py` | yes | Same action: run Python code. |
| `colab exec run` | `colab run script` | yes | Script execution belongs with other run/setup work. |
| `colab exec nb` | `colab run notebook` | yes | Notebook execution is a run command. |
| `colab env install` | `colab run pip install` | yes | Package setup belongs under `run pip`. |
| `colab env freeze` | `colab run pip freeze` | yes | Same package surface. |
| `colab env restore` | `colab run pip restore` | yes | Same package surface. |
| `colab run install` | `colab run pip install` | yes | Hidden migration alias. |
| `colab run freeze` | `colab run pip freeze` | yes | Hidden migration alias. |
| `colab run restore` | `colab run pip restore` | yes | Hidden migration alias. |
| `colab mount drive` | `colab fs drive mount` | yes | Drive is filesystem work. |
| `colab mount list` | `colab fs drive status` | yes | Users want Drive state, not a generic mount dump. |
| `colab runtime info` | `colab status runtime` | yes | Runtime reads belong under status. |
| `colab runtime gpu` | `colab status runtime --gpu` | yes | Same read-only question. |
| `colab restart-kernel` | `colab session kernel restart --yes` | no public alias | Kernel control belongs under session and requires confirmation. |
| `colab log` | `colab session logs` | yes | Logs belong to a session. |
| `colab pay` | `colab pay` | public | Opens the Colab billing / compute units page. |
| `colab version` | `colab version` or `colab --version` | public | Version is pipeable and short. |
| `colab update` | `colab update` | public | Checks for updates; install needs explicit `--install --yes`. |
| `colab drivemount` | `colab fs drive mount` | no public alias | Drive is filesystem work. |
| `colab install` | `colab run pip install` | no public alias | Package setup belongs under `run pip`. |
| `colab tools list` | `colab ai tools list` | yes | Agent/tool surfaces belong under AI. |
| `colab tools inspect` | `colab ai tools inspect` | yes | Same metadata. |
| `colab agent ...` | `colab ai ...` | yes | One public agent-facing command space. |
| `colab config get` | `colab settings get` | yes | Local config belongs under settings. |
| `colab doctor` | `colab status check` | yes | One health-check surface. |
| `colab session status` | `colab status session` | yes | Session status should not live in two visible places. |
| `colab slurp ...` | `colab distribute recipe ...` | yes | Recipe workflow now lives under distribute. |
| `colab fleet ...` | `colab distribute pool ...` | yes | Pool planning now lives under distribute. |
| `colab ai ast ...` | `colab run ast ...` | hidden compatibility only | AST is an execution/run aid, not the primary AI command. |
| `colab continue ...` | `colab continue ...` after experiment enable | n/a | Useful but optional checkpoint/replay feature. |
| `colab release ...` | `colab settings dev release ...` | no public alias | Private maintainer helper behind feature-gated dev tools. |
| snake_case skill names | dotted tool names | internal alias only | Human output never shows old raw names. |

## Drive Mount

`mount drive` is only a hidden migration path. It prints:

```text
moved: use `colab fs drive mount`
```

The implementation lives behind `fs drive mount`. It runs Drive mount through a Colab kernel cell and maps expected kernel/browser-approval failures to short errors.
