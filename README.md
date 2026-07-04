# colab-cli

Rust CLI for Google Colab sessions, code execution, file transfer, Drive, runtime status, and agent-facing code tools.

Running `colab-cli` with no command prints help. It does not open a menu.

## Quick Start

```sh
colab-cli session new --name train --gpu T4
colab-cli session kernel refresh --session train
colab-cli run pkg add --session train torch transformers
colab-cli run script train.py --session train -- --epochs 3
colab-cli fs pull /content/checkpoints ./checkpoints
colab-cli session stop --session train
```

Agent-facing tools are visible through `ai`:

```sh
colab-cli ai tools list
colab-cli ai tools inspect recipe.plan
```

Experimental spaces are off by default:

```sh
colab-cli settings experiments
colab-cli settings experiments set distribute true
colab-cli settings experiments set continue true
```

`continue` is checkpoint/replay metadata. It does not move live Python process memory between runtimes.

## Default Help

```text
Google Colab from the terminal

Usage: colab-cli [OPTIONS] <COMMAND>

Commands:
  session      Manage Colab sessions
  run          Run code and prepare runtimes
  fs           Files, sync, and Drive
  status       State, health, and runtime info
  ai           Agent, MCP, and code tools
  auth         Google account profiles
  settings     Config, experiments, support, billing, and UI
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
colab-cli settings ui set color auto
colab-cli settings ui set color always
colab-cli settings ui set color never
```

`--no-color` remains as a one-shot override.

Debugging uses SSH-style verbosity:

```sh
colab-cli -v status
colab-cli -vv fs drive mount
colab-cli --json -v status
```

Debug lines go to stderr and are redacted. See [Debugging](docs/debugging.md).
Kernel selection and language-aware packages are covered in [Kernel](docs/kernel.md).

## Command Space

```text
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli status session --name trainer
colab-cli session stop --session trainer
colab-cli session url --session trainer --open

colab-cli run script train.py --session trainer -- arg1 arg2
colab-cli run script train.py --ast --session trainer
colab-cli run py --session trainer --code "print(1)"
colab-cli run notebook notebook.ipynb --session trainer --out executed.ipynb
colab-cli run notebook notebook.ipynb --ast --session trainer
colab-cli run repl --session trainer
colab-cli run shell --session trainer
echo "print('hello')" | colab-cli run repl --session trainer
echo "echo HELLO" | colab-cli run shell --session trainer
colab-cli session kernel list --session trainer
colab-cli session kernel select python3 --session trainer
colab-cli session kernel restart --session trainer --yes
colab-cli run pkg add numpy pandas --session trainer
colab-cli run pip install torch transformers --session trainer
colab-cli run pip install -r requirements.txt --session trainer
colab-cli run pip freeze --session trainer
colab-cli run pip restore requirements.txt --session trainer

colab-cli fs ls /content
colab-cli fs push ./data.csv /content/data.csv
colab-cli fs pull /content/out ./out
colab-cli fs sync ./src /content/src --dry-run
colab-cli fs changed ./src /content/src
colab-cli fs drive mount --session trainer --path /content/drive
colab-cli fs drive status --session trainer

colab-cli status
colab-cli status runtime --backend
colab-cli status runtime --fit llama-7b

colab-cli ai tools list
colab-cli run ast file.py
colab-cli ai code deps file.py
colab-cli settings experiments
```

Compatibility groups and old aliases parse where migration is cheap. They stay hidden from normal help and print a migration hint when used.

## Experimental Distribute

`distribute` replaces the old public `slurp` and `fleet` surfaces.

- `recipe` is the tiny TOML workflow config.
- `pool` is approved runtime-pool planning.
- `shard` is safe chunk planning.

```sh
colab-cli settings experiments set distribute true
colab-cli distribute recipe init
colab-cli distribute recipe explain
colab-cli distribute pool plan --config cocli.recipe.toml --cost
colab-cli distribute run --dry-run
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
