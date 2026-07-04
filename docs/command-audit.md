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
| `colab-cli exec py` | `colab-cli run py` | yes | Same action: run Python code. |
| `colab-cli exec run` | `colab-cli run script` | yes | Script execution belongs with other run/setup work. |
| `colab-cli exec nb` | `colab-cli run notebook` | yes | Notebook execution is a run command. |
| `colab-cli env install` | `colab-cli run pip install` | yes | Package setup belongs under `run pip`. |
| `colab-cli env freeze` | `colab-cli run pip freeze` | yes | Same package surface. |
| `colab-cli env restore` | `colab-cli run pip restore` | yes | Same package surface. |
| `colab-cli run install` | `colab-cli run pip install` | yes | Hidden migration alias. |
| `colab-cli run freeze` | `colab-cli run pip freeze` | yes | Hidden migration alias. |
| `colab-cli run restore` | `colab-cli run pip restore` | yes | Hidden migration alias. |
| `colab-cli mount drive` | `colab-cli fs drive mount` | yes | Drive is filesystem work. |
| `colab-cli mount list` | `colab-cli fs drive status` | yes | Users want Drive state, not a generic mount dump. |
| `colab-cli runtime info` | `colab-cli status runtime` | yes | Runtime reads belong under status. |
| `colab-cli runtime gpu` | `colab-cli status runtime --gpu` | yes | Same read-only question. |
| `colab-cli restart-kernel` | `colab-cli session kernel restart --yes` | no public alias | Kernel control belongs under session and requires confirmation. |
| `colab-cli log` | `colab-cli session logs` | yes | Logs belong to a session. |
| `colab-cli pay` | `colab-cli settings billing open` | no public alias | Billing helpers belong under settings. |
| `colab-cli version` | `colab-cli status version` or `colab-cli --version` | no public alias | Version is status/about information. |
| `colab-cli update` | `colab-cli settings update check` | no public alias | Updates are local settings/about maintenance. |
| `colab-cli drivemount` | `colab-cli fs drive mount` | no public alias | Drive is filesystem work. |
| `colab-cli install` | `colab-cli run pip install` | no public alias | Package setup belongs under `run pip`. |
| `colab-cli tools list` | `colab-cli ai tools list` | yes | Agent/tool surfaces belong under AI. |
| `colab-cli tools inspect` | `colab-cli ai tools inspect` | yes | Same metadata. |
| `colab-cli agent ...` | `colab-cli ai ...` | yes | One public agent-facing command space. |
| `colab-cli config get` | `colab-cli settings get` | yes | Local config belongs under settings. |
| `colab-cli doctor` | `colab-cli status check` | yes | One health-check surface. |
| `colab-cli session status` | `colab-cli status session` | yes | Session status should not live in two visible places. |
| `colab-cli slurp ...` | `colab-cli distribute recipe ...` | yes | Recipe workflow now lives under distribute. |
| `colab-cli fleet ...` | `colab-cli distribute pool ...` | yes | Pool planning now lives under distribute. |
| `colab-cli ai ast ...` | `colab-cli run ast ...` | hidden compatibility only | AST is an execution/run aid, not the primary AI command. |
| `colab-cli continue ...` | `colab-cli continue ...` after experiment enable | n/a | Useful but optional checkpoint/replay feature. |
| `colab-cli release ...` | `colab-cli settings dev release ...` | no public alias | Private maintainer helper behind feature-gated dev tools. |
| snake_case skill names | dotted tool names | internal alias only | Human output never shows old raw names. |

## Drive Mount

`mount drive` is only a hidden migration path. It prints:

```text
moved: use `colab-cli fs drive mount`
```

The implementation lives behind `fs drive mount`. It runs Drive mount through a Colab kernel cell and maps expected kernel/browser-approval failures to short errors.
