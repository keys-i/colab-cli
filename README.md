# colab

Rust CLI for Google Colab sessions, code execution, file transfer, Drive, runtime status, and agent-facing code tools.

The Cargo package is `colab-cli`. The primary installed binary is `colab`; `colab-cli` remains a compatibility alias.

Running `colab` with no command prints help. It does not open a menu.

## Quick Start

```sh
colab session new --name train --gpu T4
colab session kernel refresh --session train
colab run pkg add --session train torch transformers
colab run script train.py --session train -- --epochs 3
colab fs download /content/checkpoints ./checkpoints
colab session stop --session train
```

Agent-facing tools are visible through `ai`:

```sh
colab ai tools list
colab ai tools inspect runtime.inspect
```

Experimental spaces are off by default:

```sh
colab settings experiments
colab settings experiments set distribute true
colab settings experiments set continue true
```

`continue` is checkpoint/replay metadata. It does not move live Python process memory between runtimes.

## Default Help

```text
Google Colab from the terminal

Usage: colab [OPTIONS] <COMMAND>

Commands:
  session      Manage Colab sessions
  run          Run code on Colab
  fs           Files, sync, and Drive
  status       Session and runtime status
  auth         Sign in and inspect credentials
  log          View and export history
  settings     Config, UI, support, and experiments
  ai           Agent-facing tools
  update       Check or install updates
  version      Show version
  pay          Open Colab billing / compute units page
  completions  Generate shell completions

Options:
  -q, --quiet
      --json
  -v, --verbose
      --no-color
      --bell
  -h, --help
  -V, --version
```

`--color` is not part of normal help. Set colour mode with:

```sh
colab settings ui set color auto
colab settings ui set color always
colab settings ui set color never
```

`--no-color` remains as a one-shot override.

Debugging uses SSH-style verbosity:

```sh
colab -v status
colab -vv fs drive mount
colab --json -v status
```

Debug lines go to stderr and are redacted. See [Debugging](docs/debugging.md).
Kernel selection and language-aware packages are covered in [Kernel](docs/kernel.md).

## Command Space

```text
colab session new --name trainer --gpu A100
colab session list
colab status session --name trainer
colab session stop --session trainer
colab session url --session trainer --open

colab run script train.py --session trainer -- arg1 arg2
colab run script train.py --ast --session trainer
colab run py --session trainer --code "print(1)"
colab run notebook notebook.ipynb --session trainer --out executed.ipynb
colab run notebook notebook.ipynb --ast --session trainer
colab run repl --session trainer
colab run shell --session trainer
echo "print('hello')" | colab run repl --session trainer
echo "echo HELLO" | colab run shell --session trainer
colab session kernel list --session trainer
colab session kernel select python3 --session trainer
colab session kernel restart --session trainer --yes
colab run pkg add numpy pandas --session trainer
colab run pip install torch transformers --session trainer
colab run pip install -r requirements.txt --session trainer
colab run pip freeze --session trainer
colab run pip restore requirements.txt --session trainer

colab fs ls /content
colab fs upload ./data.csv /content/data.csv
colab fs download /content/out ./out
colab fs sync ./src /content/src --dry-run
colab fs changed ./src /content/src
colab fs drive mount --session trainer --path /content/drive
colab fs drive status --session trainer

colab status
colab status runtime --backend
colab status runtime --fit llama-7b

colab ai tools list
colab ai code deps file.py
colab settings experiments
```

Compatibility groups and old aliases parse where migration is cheap. They stay hidden from normal help and print a migration hint when used.

## Experimental Distribute

`distribute` replaces the old public `slurp` and `fleet` surfaces.

- `recipe` is the tiny TOML workflow config.
- `pool` is approved runtime-pool planning.
- `shard` is safe chunk planning.

```sh
colab settings experiments set distribute true
colab distribute recipe init
colab distribute recipe explain
colab distribute pool plan --config cocli.recipe.toml --cost
colab distribute run --dry-run
```

`slurp.toml` is still read for old projects. New docs use `cocli.recipe.toml`.

No distribute feature may rotate accounts to bypass Colab limits. Multi-login is locked unless `distribute` is enabled.

## Layout

```text
src/
  main.rs
  lib.rs
  cocli/
    cli/
    auth/
    session/
    exec/
    fs/
    runtime/
    slurp/      internal recipe parser
    fleet/      internal distribute planner
    agent/
    continue/
    tools/
    config/
    release/    private maintainer helpers
    ui/
    util/
```

Unsafe code is forbidden by package lints.

## Build

```sh
cargo build
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
```

## Docs

- [Commands](docs/commands.md)
- [Settings](docs/settings.md)
- [Auth](docs/auth.md)
- [Run](docs/run.md)
- [Logs](docs/logs.md)
- [AI](docs/ai.md)
- [AST observer](docs/ast-observer.md)
- [Distribute](docs/distribute.md)
- [Google Colab CLI map](docs/google-colab-cli-map.md)
- [Colabtools feature map](docs/colabtools-feature-map.md)
- [Command audit](docs/command-audit.md)
- [Prune and merge report](docs/prune-and-merge-report.md)
- [Output style](docs/output-style.md)
- [Debugging](docs/debugging.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Use cases](docs/use-cases.md)
- [Perf pass](docs/perf-pass.md)
- [Claims ledger](docs/claims-ledger.md)
- [QA](docs/qa.md)
- [Maintainer notes](docs/maintainer.md)
- [Live testing](docs/live-testing.md)
- [Plan](plan.md)
