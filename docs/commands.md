# Commands

Command shape:

```text
colab-cli <space> <command> <flags>
```

Public spaces: `session`, `run`, `fs`, `status`, `continue`, `slurp`, `fleet`, `auth`, `settings`, `completions`.

Hidden aliases exist for one migration cycle where they are cheap. They do not appear in normal help.

## Session

```sh
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli session last
colab-cli session stop --session trainer
colab-cli session url --session trainer --open
```

Use `status session` for session details.

## Run

```sh
colab-cli run py --session trainer --code "print(1)"
colab-cli run script train.py --session trainer -- --epochs 3
colab-cli run notebook report.ipynb --session trainer --out report.out.ipynb
colab-cli run repl --session trainer
colab-cli run shell --session trainer
colab-cli run install torch transformers --session trainer
colab-cli run install -r requirements.txt --session trainer
colab-cli run freeze --session trainer
colab-cli run restore requirements.txt --session trainer
colab-cli run last --confirm
```

`run script` executes the path on the remote runtime. Push local files first with `fs push`.

## Fs

```sh
colab-cli fs ls /content
colab-cli fs push ./data.csv /content/data.csv
colab-cli fs pull /content/out ./out
colab-cli fs rm /content/tmp --recursive --yes
colab-cli fs sync ./src /content/src --dry-run
colab-cli fs diff ./src /content/src
colab-cli fs changed ./src /content/src
colab-cli fs drive mount --session trainer --path /content/drive
colab-cli fs drive status --session trainer
colab-cli fs drive list --session trainer
colab-cli fs drive unmount --session trainer
colab-cli fs drive path --session trainer
```

`fs rm` requires `--yes`. `fs sync` is dry-run planning in this release.

Drive mount runs through a Colab kernel cell. If the session has not been opened in a browser yet, run:

```sh
colab-cli session url --session trainer --open
```

## Status

```sh
colab-cli status
colab-cli status quick
colab-cli status check
colab-cli status session --name trainer
colab-cli status runtime --all
colab-cli status runtime --backend
colab-cli status runtime --gpu
colab-cli status runtime --tpu
colab-cli status runtime --versions
colab-cli status runtime --fit llama-7b
colab-cli status auth
colab-cli status fs
colab-cli status drive
colab-cli status slurp
colab-cli status fleet
colab-cli status run
colab-cli status paths
```

`status quick` is the short local check. It should print one useful next action.

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

## Slurp And Fleet

```sh
colab-cli slurp init
colab-cli slurp explain
colab-cli fleet plan --config slurp.toml --cost
```

Fleet execution is deferred. Planning and compliance checks are local and do not bypass Colab rules.

## Settings

```sh
colab-cli settings get
colab-cli settings set ui.fun true
colab-cli settings path
colab-cli settings edit
colab-cli settings ui get
colab-cli settings ui set animations false
colab-cli settings ui preview
colab-cli settings skills list
colab-cli settings skills inspect slurp.plan
colab-cli settings skills run slurp.plan --json-input '{}'
```

Skills are optional agent/tool surfaces. Core work stays in normal commands.

## Migration Aliases

```text
exec      -> run
env       -> run install/freeze/restore
mount     -> fs drive
runtime   -> status runtime
tools     -> settings skills
config    -> settings
doctor    -> status check
```
