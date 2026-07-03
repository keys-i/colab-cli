# colab-cli

Rust CLI for Google Colab sessions, execution, file transfer, continuation bundles, and agent-friendly tool discovery.

## Quick Start

```sh
colab-cli session new --name train --gpu T4
colab-cli env install --session train torch transformers
colab-cli exec run train.py --session train -- --epochs 3
colab-cli continue save --session train --name train-run
colab-cli fs pull /content/checkpoints ./checkpoints
colab-cli session stop --name train
```

Agent-friendly plan surface:

```sh
colab-cli agent tools
colab-cli agent plan "run notebook.ipynb on an L4 and pull ./outputs"
colab-cli agent run plan.toml --confirm
```

Continuation bundle:

```sh
colab-cli continue save --session train --name before-long-run
colab-cli continue export before-long-run --out before-long-run.cocli
colab-cli continue import before-long-run.cocli
colab-cli continue resume before-long-run --new-runtime --gpu L4
```

Continuation restores files, metadata, mounts, environment plans, and pending command steps. It does not move live Python process memory between unrelated runtimes.

## Command Space

```text
colab-cli session new --name trainer --gpu A100
colab-cli session list
colab-cli session status --session trainer
colab-cli session stop --session trainer
colab-cli session url --session trainer --open

colab-cli exec run train.py --session trainer -- arg1 arg2
colab-cli exec py --session trainer --code "print(1)"
colab-cli exec nb notebook.ipynb --session trainer --out executed.ipynb
colab-cli exec repl --session trainer
colab-cli exec shell --session trainer

colab-cli fs ls /content
colab-cli fs push ./data.csv /content/data.csv
colab-cli fs pull /content/out ./out
colab-cli fs rm /content/tmp --recursive --yes
colab-cli fs sync ./src /content/src --dry-run
colab-cli fs diff ./src /content/src

colab-cli mount drive --session trainer --path /content/drive
colab-cli env install torch transformers --session trainer
colab-cli runtime backend-info
colab-cli tools list
colab-cli doctor
```

Compatibility groups `server` and `file` still parse. Hidden aliases cover cheap old `colab new`, `colab sessions`, `colab upload`, and `colab download` forms with migration hints.

## Workspace

```text
crates/
  colab-cli/        binary crate and existing Colab HTTP/client code
  cocli-core/       config, session lookup, color and terminal bell policy
  cocli-colab/      Colab request helpers and safe command snippets
  cocli-tools/      built-in tool registry and JSON tool output
  cocli-protocol/   continuation and tool protocol structs
  cocli-fs/         manifests, diff planning, chunk planning
```

Unsafe code is forbidden by workspace lints.

## Build

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

Release builds use thin LTO, one codegen unit, symbol stripping, `opt-level = 3`, and `panic = "abort"`.

## Docs

- [Commands](docs/commands.md)
- [Architecture](docs/architecture.md)
- [Continuation](docs/continuation.md)
- [Tools](docs/tools.md)
- [Performance](docs/performance.md)
- [Publishing](docs/publishing.md)
- [Migration from google-colab-cli](docs/migration-from-google-colab-cli.md)
- [Research notes](docs/research-notes.md)
- [Decisions](docs/decisions.md)
