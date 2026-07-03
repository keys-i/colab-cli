# Commands

Command shape:

```text
colab-cli <major-space> <command> <flags>
```

Major spaces: `session`, `exec`, `fs`, `mount`, `env`, `runtime`, `tools`, `agent`, `continue`, `config`, `doctor`.

## Session

```sh
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli session status --session trainer
colab-cli session stop --session trainer
colab-cli session url --session trainer --open
```

`server` still exists as a hidden compatibility group for the older Rust CLI.

## Exec

```sh
colab-cli exec run train.py --session trainer -- arg1 arg2
colab-cli exec py --session trainer --code "print(1)"
colab-cli exec nb notebook.ipynb --session trainer --out executed.ipynb
colab-cli exec repl --session trainer
colab-cli exec shell --session trainer
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
```

`fs rm` requires `--yes`. `fs sync` is dry-run planning in this release.

## Continue

```sh
colab-cli continue save --session trainer --name run-a
colab-cli continue inspect run-a
colab-cli continue export run-a --out run-a.cocli
colab-cli continue import run-a.cocli
colab-cli continue resume run-a --new-runtime --gpu L4
colab-cli continue clean --older-than 7d
```

Resume replays manifest steps. It does not restore live Python variables.
