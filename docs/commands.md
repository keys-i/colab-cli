# Commands

Command shape:

```text
colab <space> <command> <flags>
```

Default public spaces: `session`, `run`, `fs`, `status`, `auth`, `log`, `settings`, `ai`, `update`, `version`, `pay`, `completions`.

Experimental spaces are hidden until explicitly enabled: `continue`, `distribute`.

Hidden aliases exist for one migration cycle where they are cheap. They do not appear in normal help.

## Session

```sh
colab session new --name trainer --gpu A100
colab session list
colab session last
colab session stop --session trainer
colab session url --session trainer --open
colab session kernel list --session trainer
colab session kernel current --session trainer
colab session kernel select python3 --session trainer
colab session kernel specs --session trainer
colab session kernel interrupt --session trainer
colab session kernel restart --session trainer --yes
```

Use `status session` for session details.
See [kernel.md](kernel.md) for kernel selection and language-aware package tooling.

## Run

```sh
colab run py --session trainer --code "print(1)"
colab run script train.py --session trainer -- --epochs 3
colab run script train.py --ast --session trainer
colab run notebook report.ipynb --session trainer --out report.out.ipynb
colab run notebook report.ipynb --ast --session trainer
colab run repl --session trainer
colab run shell --session trainer
colab run ast train.py
colab run code --session trainer --code "1 + 1"
colab run pkg add numpy pandas --session trainer
colab run pkg list --session trainer
colab run pip install torch transformers --session trainer
colab run pip install -r requirements.txt --session trainer
colab run pip freeze --session trainer
colab run pip restore requirements.txt --session trainer
colab run pip check --session trainer
colab run pip list --session trainer
```

`run script` executes the path on the remote runtime. Upload local files first with `fs upload`.

`--ast` prints a local code outline before execution when the AST observer experiment is enabled.

`run repl` uses the attached Jupyter kernel, not a raw remote `python` process.
`run shell` uses the Colab `/colab/tty` PTY websocket where supported, not
Jupyter `/api/terminals` by default.

`run pkg` follows the active kernel. `run pip` is Python-specific and is not
shown as primary help when cached metadata says the active kernel is Julia or R.

## Fs

```sh
colab fs ls /content
colab fs upload ./data.csv /content/data.csv
colab fs download /content/out ./out
colab fs rm /content/tmp --recursive --yes
colab fs sync ./src /content/src --dry-run
colab fs diff ./src /content/src
colab fs changed ./src /content/src
colab fs drive mount --session trainer --path /content/drive
colab fs drive status --session trainer
colab fs drive list --session trainer
colab fs drive unmount --session trainer
colab fs drive path --session trainer
```

`fs rm` requires `--yes`. `fs sync` is dry-run planning in this release.

Drive mount runs through a Colab kernel cell. If the session has not been opened in a browser yet, run:

```sh
colab session url --session trainer --open
```

## Status

```sh
colab status
colab status quick
colab status check
colab status session --name trainer
colab status runtime --all
colab status runtime --backend
colab status runtime --gpu
colab status runtime --tpu
colab status runtime --versions
colab status runtime --fit llama-7b
colab status auth
colab status fs
colab status drive
colab status kernel
colab status kernel --all
colab status kernel --refresh
colab status run
colab status paths
colab status version
```

`status` is a cheap local check by default. It prints one `fix:` line only when something needs attention.

## Auth

```sh
colab auth login --method oauth2
colab auth login --method adc
colab auth status
colab auth list
colab auth use --name work
colab auth logout work
colab auth export-redacted
```

OAuth2 and ADC are explicit choices. Tokens are not printed.

## Log, Update, Version, Pay

```sh
colab log
colab log list
colab log show --tail 50
colab log export --format md --out history.md
colab update
colab update --install --yes
colab version
colab pay --dry-run
```

`log` reports local history that cocli has actually recorded. It does not invent remote server logs. `update --install` never runs without an explicit install flag and confirmation.

## Continue

Disabled by default:

```text
experimental feature disabled: continue
enable: colab settings experiments
```

When enabled:

```sh
colab continue save --session trainer --name run-a
colab continue inspect run-a
colab continue export run-a --out run-a.cocli
colab continue import run-a.cocli
colab continue resume run-a --dry-run
colab continue resume run-a --confirm
colab continue last
colab continue clean --older-than 7d
```

Resume replays manifest steps. It does not restore live Python variables.

## Distribute

Disabled by default:

```text
experimental feature disabled: distribute
enable: colab settings experiments
```

When enabled:

```sh
colab distribute plan
colab distribute status
colab distribute explain
colab distribute run --dry-run
colab distribute run --confirm
colab distribute resume
colab distribute clean

colab distribute recipe init
colab distribute recipe check
colab distribute recipe explain
colab distribute recipe run --dry-run
colab distribute recipe run --confirm

colab distribute pool plan
colab distribute pool status
colab distribute pool cost
colab distribute pool logs

colab distribute shard plan
colab distribute shard run --dry-run
colab distribute shard resume
```

The old hidden `slurp` alias maps to `distribute recipe`. The old hidden `fleet` alias maps to `distribute pool`.

Distribute must not bypass Colab rules or quotas. Multi-login is locked unless distribute is enabled.

## AI

```sh
colab ai
colab ai tools list
colab ai tools inspect runtime.inspect
colab ai code explain file.py
colab ai code deps file.py
colab ai plan "summarise a workflow"
colab ai audit plan.toml
colab ai explain plan.toml
colab ai run plan.toml --confirm
colab ai mcp
colab ai mcp serve --stdio
```

`ai tools list` is read-only and available by default. Distribute, continue, MCP, and AST tool rows appear only after their experiments are enabled. AST is shown under `run ast` and `run script --ast`. MCP serving, plan drafting, and `ai run` are disabled until enabled under `settings experiments`. Plans are inspectable and do not execute hidden Colab work.

## Settings

```sh
colab settings get
colab settings set ui.fun true
colab settings path
colab settings edit
colab settings ui get
colab settings ui set color auto
colab settings ui set animations false
colab settings ui reset
colab settings ui preview
colab settings experiments
colab settings experiments get
colab settings experiments set distribute true
colab settings experiments set continue true
colab settings experiments reset
colab settings skills list
colab settings skills inspect runtime.inspect
colab settings skills run agent.audit --json-input '{}'
colab settings support bug-report
colab settings about
colab update
colab update --install --yes
colab pay --dry-run
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
