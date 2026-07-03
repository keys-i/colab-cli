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
| `colab-cli tools list` | `colab-cli settings skills list` | clap, README, tools docs, tests | yes | The registry is a user setting surface, not a top-level tool. |
| `colab-cli tools inspect` | `colab-cli settings skills inspect` | clap, tools docs, tests | yes | Same skill metadata. |
| `colab-cli tools run` | `colab-cli settings skills run` | clap, tools docs | yes | Same dry-run command plan. |
| `colab-cli config get` | `colab-cli settings get` | clap, commands docs | yes | Local config belongs under settings. |
| `colab-cli config set` | `colab-cli settings set` | clap, README, commands docs | yes | Local config belongs under settings. |
| `colab-cli config path` | `colab-cli settings path` | clap, README, commands docs, tests | yes | Same path lookup. |
| `colab-cli config open` | `colab-cli settings edit` | clap, tests | yes | Editor action reads better as settings edit. |
| `colab-cli doctor quick` | `colab-cli status quick` | clap, README, commands docs, tests | yes | Health checks are status reads. |
| `colab-cli doctor` | `colab-cli status check` | clap, tests | yes | One health-check surface. |
| `colab-cli session status` | `colab-cli status session` | clap, README, commands docs | yes | Session status should not live in two visible places. |
| `session_new` | `session.new` | skill registry, tools docs, tests | internal alias only | Dot names are easier to read in JSON and tables. |
| `exec_python` | `run.python` | skill registry, tools docs, tests | internal alias only | Old snake name still resolves for one cycle. |
| `env_install` | `run.install` | skill registry, tools docs | internal alias only | Matches the run command surface. |
| `runtime_info` | `runtime.info` | skill registry, tools docs | internal alias only | Skill names describe the action, not the old command space. |
| `doctor` skill | `status.check` | skill registry, tools docs | internal alias only | Health checks moved under status. |

`auth`, `agent`, `bug-report`, `server`, `file`, and shell completions are hidden from top-level help. They remain parseable because there is no smaller replacement for every existing flow yet.
