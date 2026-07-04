# colab-cli

Rust CLI for Google Colab sessions, execution, file transfer, continuation bundles, Slurp plans, local diagnostics, and agent-facing tool metadata.

Running `colab-cli` with no command opens a small launcher when stdin and stdout are interactive. Scripts should call explicit commands and use `--json` when they need machine output.

## Quick Start

```sh
colab-cli session new --name train --gpu T4
colab-cli run install --session train torch transformers
colab-cli run script train.py --session train -- --epochs 3
colab-cli continue save --session train --name train-run
colab-cli continue resume train-run --dry-run
colab-cli fs pull /content/checkpoints ./checkpoints
colab-cli session stop --name train
```

Agent-facing tools are visible through `ai`:

```sh
colab-cli ai tools list
colab-cli ai tools inspect slurp.plan
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
colab-cli status session --name trainer
colab-cli session stop --session trainer
colab-cli session url --session trainer --open

colab-cli run script train.py --session trainer -- arg1 arg2
colab-cli run py --session trainer --code "print(1)"
colab-cli run notebook notebook.ipynb --session trainer --out executed.ipynb
colab-cli run repl --session trainer
colab-cli run shell --session trainer

colab-cli fs ls /content
colab-cli fs push ./data.csv /content/data.csv
colab-cli fs pull /content/out ./out
colab-cli fs rm /content/tmp --recursive --yes
colab-cli fs sync ./src /content/src --dry-run
colab-cli fs changed ./src /content/src

colab-cli fs drive mount --session trainer --path /content/drive
colab-cli fs drive status --session trainer
colab-cli fs drive list --session trainer
colab-cli fs drive unmount --session trainer
colab-cli run install torch transformers --session trainer
colab-cli status runtime --backend
colab-cli status runtime --fit llama-7b
colab-cli ai tools list
colab-cli settings experiments
colab-cli status quick
```

Compatibility groups and old aliases parse where migration is cheap. They stay hidden from normal help and print a migration hint when used.

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
    agent/
    continue/
    tools/
    config/
    release/    internal naming helpers
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

Research notes live in [docs/research.md](docs/research.md). The current build plan lives in [plan.md](plan.md).

- [Commands](docs/commands.md)
- [Architecture](docs/architecture.md)
- [Research](docs/research.md)
- [Plan](plan.md)
- [Refactor map](docs/refactor-map.md)
- [Prune report](docs/prune-report.md)
- [Command audit](docs/command-audit.md)
- [Drive](docs/drive.md)
- [UI](docs/ui.md)
- [Settings](docs/settings.md)
- [AI](docs/ai.md)
- [MCP](docs/mcp.md)
- [Skills](docs/skills.md)
- [Feature test plan](docs/feature-test-plan.md)
- [Live testing](docs/live-testing.md)
- [Easter eggs](docs/easter-eggs.md)
- [Continuation](docs/continuation.md)
- [Performance](docs/performance.md)
- [Benchmark plan](docs/benchmark-plan.md)
- [Benchmark results](docs/benchmark-results.md)
- [Claims ledger](docs/claims-ledger.md)
- [Competitor matrix](docs/competitor-matrix.md)
- [Output style](docs/output-style.md)
- [Perf pass](docs/perf-pass.md)
- [QA](docs/qa.md)
- [Usability study](docs/usability-study.md)
- [Publishing](docs/publishing.md)
- [CI/CD](docs/ci-cd.md)
- [Migration from google-colab-cli](docs/migration-from-google-colab-cli.md)
- [Research notes](docs/research-notes.md)
- [Decisions](docs/decisions.md)
