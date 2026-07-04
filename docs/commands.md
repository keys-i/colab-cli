# Commands

Command shape:

```text
colab-cli <space> <command> <flags>
```

Default public spaces: `session`, `run`, `fs`, `status`, `ai`, `auth`, `settings`, `completions`.

Experimental spaces are hidden until explicitly enabled: `continue`, `distribute`.

Hidden aliases exist for one migration cycle where they are cheap. They do not appear in normal help.

## Session

```sh
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli session last
colab-cli session stop --session trainer
colab-cli session url --session trainer --open
colab-cli session kernel list --session trainer
colab-cli session kernel current --session trainer
colab-cli session kernel select python3 --session trainer
colab-cli session kernel specs --session trainer
colab-cli session kernel interrupt --session trainer
colab-cli session kernel restart --session trainer --yes
```

Use `status session` for session details.
See [kernel.md](kernel.md) for kernel selection and language-aware package tooling.

## Run

```sh
colab-cli run py --session trainer --code "print(1)"
colab-cli run script train.py --session trainer -- --epochs 3
colab-cli run script train.py --ast --session trainer
colab-cli run notebook report.ipynb --session trainer --out report.out.ipynb
colab-cli run notebook report.ipynb --ast --session trainer
colab-cli run repl --session trainer
colab-cli run shell --session trainer
colab-cli run ast train.py
colab-cli run code --session trainer --code "1 + 1"
colab-cli run pkg add numpy pandas --session trainer
colab-cli run pkg list --session trainer
colab-cli run pip install torch transformers --session trainer
colab-cli run pip install -r requirements.txt --session trainer
colab-cli run pip freeze --session trainer
colab-cli run pip restore requirements.txt --session trainer
colab-cli run pip check --session trainer
colab-cli run pip list --session trainer
colab-cli run julia pkg add CSV DataFrames --session trainer
colab-cli run r pkg install dplyr --session trainer
colab-cli run last --confirm
```

`run script` executes the path on the remote runtime. Push local files first with `fs push`.

`--ast` prints a local code outline before execution when the AST observer experiment is enabled.

`run repl` uses the attached Jupyter kernel, not a raw remote `python` process.
`run shell` uses the Colab `/colab/tty` PTY websocket where supported, not
Jupyter `/api/terminals` by default.

`run pkg` follows the active kernel. `run pip` is Python-specific and is not
shown as primary help when cached metadata says the active kernel is Julia or R.

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
colab-cli status kernel
colab-cli status kernel --all
colab-cli status kernel --refresh
colab-cli status run
colab-cli status paths
colab-cli status version
```

`status` is a cheap local check by default. It prints one `fix:` line only when something needs attention.

## Continue

Disabled by default:

```text
experimental feature disabled: continue
enable: colab-cli settings experiments
```

When enabled:

```sh
colab-cli continue save --session trainer --name run-a
colab-cli continue inspect run-a
colab-cli continue export run-a --out run-a.cocli
colab-cli continue import run-a.cocli
colab-cli continue resume run-a --dry-run
colab-cli continue resume run-a --confirm
colab-cli continue last
colab-cli continue clean --older-than 7d
```

Resume replays manifest steps. It does not restore live Python variables.

## Distribute

Disabled by default:

```text
experimental feature disabled: distribute
enable: colab-cli settings experiments
```

When enabled:

```sh
colab-cli distribute plan
colab-cli distribute status
colab-cli distribute explain
colab-cli distribute run --dry-run
colab-cli distribute run --confirm
colab-cli distribute resume
colab-cli distribute clean

colab-cli distribute recipe init
colab-cli distribute recipe check
colab-cli distribute recipe explain
colab-cli distribute recipe run --dry-run
colab-cli distribute recipe run --confirm

colab-cli distribute pool plan
colab-cli distribute pool status
colab-cli distribute pool cost
colab-cli distribute pool logs

colab-cli distribute shard plan
colab-cli distribute shard run --dry-run
colab-cli distribute shard resume
```

The old hidden `slurp` alias maps to `distribute recipe`. The old hidden `fleet` alias maps to `distribute pool`.

Distribute must not bypass Colab rules or quotas. Multi-login is locked unless distribute is enabled.

## AI

```sh
colab-cli ai
colab-cli ai tools list
colab-cli ai tools inspect recipe.plan
colab-cli ai code explain file.py
colab-cli ai code deps file.py
colab-cli ai plan "summarise a workflow"
colab-cli ai audit plan.toml
colab-cli ai explain plan.toml
colab-cli ai run plan.toml --confirm
colab-cli ai mcp
colab-cli ai mcp serve --stdio
```

`ai tools list` is read-only and available by default. AST is shown under `run ast` and `run script --ast`. MCP serving, plan drafting, and `ai run` are disabled until enabled under `settings experiments`. Plans are inspectable and do not execute hidden Colab work.

## Settings

```sh
colab-cli settings get
colab-cli settings set ui.fun true
colab-cli settings path
colab-cli settings edit
colab-cli settings ui get
colab-cli settings ui set color auto
colab-cli settings ui set animations false
colab-cli settings ui reset
colab-cli settings ui preview
colab-cli settings experiments
colab-cli settings experiments get
colab-cli settings experiments set distribute true
colab-cli settings experiments set continue true
colab-cli settings experiments reset
colab-cli settings skills list
colab-cli settings skills inspect recipe.plan
colab-cli settings skills run recipe.plan --json-input '{}'
colab-cli settings support bug-report
colab-cli settings about
colab-cli settings update check
colab-cli settings update install --yes
colab-cli settings billing open
colab-cli settings billing status
```

Skills and AI tools are optional agent/tool surfaces. Core work stays in normal commands.

## Migration Aliases

```text
exec       -> run
env        -> run pip install/freeze/restore
mount      -> fs drive
runtime    -> status runtime
tools      -> ai tools, or settings skills for legacy scripts
config     -> settings
doctor     -> status check
agent      -> ai
bug-report -> settings support bug-report
slurp      -> distribute recipe
fleet      -> distribute pool
release    -> settings dev release, private maintainer builds only
```
