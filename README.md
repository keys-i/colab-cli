# colab-cli

Rust CLI for Google Colab sessions, execution, file transfer, continuation bundles, Slurp plans, and local diagnostics.

## Quick Start

```sh
colab-cli session new --name train --gpu T4
colab-cli env install --session train torch transformers
colab-cli exec run train.py --session train -- --epochs 3
colab-cli continue save --session train --name train-run
colab-cli continue resume train-run --dry-run
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
colab-cli fs changed ./src /content/src

colab-cli mount drive --session trainer --path /content/drive
colab-cli env install torch transformers --session trainer
colab-cli runtime info --backend
colab-cli runtime fit --model llama-7b
colab-cli tools list
colab-cli doctor quick
```

Compatibility groups `server` and `file` still parse. Hidden aliases cover cheap old `colab new`, `colab sessions`, `colab upload`, and `colab download` forms with migration hints.

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
    slurp/
    fleet/
    continue/
    tools/
    config/
    doctor/
    release/
    ui/
    util/
```

One package is published: `colab-cli`. Internal modules can become crates later if a real public API needs that.

Unsafe code is forbidden by package lints.

## Build

```sh
cargo build
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
```

Release builds use thin LTO, one codegen unit, symbol stripping, `opt-level = 3`, and `panic = "abort"`.

## Release

Releases are planned with Shipyard:

```sh
cargo shipyard plan
cargo shipyard update --dry-run
cargo shipyard pr --base main
```

The scheduled workflow runs on the 1st and 16th of each month. Publishing still requires a tag and the crates.io token in the publish job. Shipyard comparison data lives in `target/shipyard/comparison-release-plz.md`; faster than release-plz is not proven until both tools are measured on the same checkout.

## Docs

- [Commands](docs/commands.md)
- [Architecture](docs/architecture.md)
- [Refactor map](docs/refactor-map.md)
- [Prune report](docs/prune-report.md)
- [Easter eggs](docs/easter-eggs.md)
- [Continuation](docs/continuation.md)
- [Tools](docs/tools.md)
- [Performance](docs/performance.md)
- [Publishing](docs/publishing.md)
- [Release](docs/release.md)
- [CI/CD](docs/ci-cd.md)
- [Migration from google-colab-cli](docs/migration-from-google-colab-cli.md)
- [Research notes](docs/research-notes.md)
- [Decisions](docs/decisions.md)
