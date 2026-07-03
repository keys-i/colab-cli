# Commands

Command shape:

```text
colab-cli <major-space> <command> <flags>
```

Major spaces: `auth`, `session`, `exec`, `fs`, `mount`, `env`, `runtime`, `slurp`, `fleet`, `tools`, `agent`, `continue`, `config`, `doctor`, `release`.

## Session

```sh
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli session status --session trainer
colab-cli session stop --session trainer
colab-cli session url --session trainer --open
colab-cli session last
```

`server` still exists as a hidden compatibility group for the older Rust CLI.
Use `--session -` to target the last local session where a command accepts `--session`.

## Exec

```sh
colab-cli exec run train.py --session trainer -- arg1 arg2
colab-cli exec py --session trainer --code "print(1)"
colab-cli exec nb notebook.ipynb --session trainer --out executed.ipynb
colab-cli exec repl --session trainer
colab-cli exec shell --session trainer
colab-cli exec last --confirm
```

`exec run` currently executes the path on the remote runtime. Push local files first with `fs push`.

## Fs

```sh
colab-cli fs ls /content
colab-cli fs push ./data.csv /content/data.csv
colab-cli fs pull /content/out ./out
colab-cli fs rm /content/tmp --recursive --yes
colab-cli fs sync ./src /content/src --dry-run
colab-cli fs diff ./src /content/src
colab-cli fs changed ./src /content/src
```

`fs rm` requires `--yes`. `fs sync` is dry-run planning in this release.

## Runtime

```sh
colab-cli runtime info
colab-cli runtime info --backend
colab-cli runtime fit --model llama-7b
```

`runtime fit` is a rough local heuristic: `probably-fits`, `tight`, `nope`, or `unknown`.

## Slurp And Fleet

```sh
colab-cli slurp init
colab-cli slurp explain
colab-cli slurp doctor
colab-cli fleet plan --config slurp.toml --cost
```

Fleet execution is deferred. Planning and compliance checks are local and do not bypass Colab rules.

## Continue

```sh
colab-cli continue save --session trainer --name run-a
colab-cli continue inspect run-a
colab-cli continue export run-a --out run-a.cocli
colab-cli continue import run-a.cocli
colab-cli continue resume run-a --dry-run
colab-cli continue resume run-a --new-runtime --gpu L4
colab-cli continue last
colab-cli continue clean --older-than 7d
```

Resume replays manifest steps. It does not restore live Python variables.

## Doctor And Config

```sh
colab-cli doctor quick
colab-cli doctor paths
colab-cli doctor mounts
colab-cli doctor env
colab-cli doctor --vibe
colab-cli config open
colab-cli bug-report
```
