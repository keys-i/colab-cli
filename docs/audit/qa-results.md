# QA Results

Date: 2026-07-04

## Commands Run So Far

```text
cargo check --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --all-features --test cli
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
cargo run --bin colab -- --help
cargo run --bin colab --
cargo run --bin colab -- ai --help
cargo run --bin colab -- agent plan x
./scripts/check-command-surface.sh
```

## Current Results

- `cargo check --workspace --all-features`: passed.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: passed.
- `cargo test --workspace --no-default-features`: passed.
- `cargo doc --workspace --all-features --no-deps`: passed.
- CLI integration tests: 32 passed with all features; 31 passed with no default features.
- Default help prints app help and exits successfully.
- `ai --help` hides gated execution/MCP/code surfaces.
- Old hidden `agent` alias no longer executes a plan.
- Settings renderer has width-bounded unit coverage.
- `auth status` is human output by default and JSON only under `--json`.

## Covered By Tests

- Default command surface.
- Hidden experimental command gates.
- JSON output has no ANSI.
- Verbose output goes to stderr.
- Settings direct persistence and state-machine behavior.
- Settings editor text is vertical and bounded at 60/80/100/140 columns.
- Drive timeout default allows human auth.
- Drive kernel-code path is used for mount cell generation.
- Shell `/colab/tty` URL shape.
- Secret redaction and gate behavior.
- AI tool catalog JSON/human cleanliness.

## Not Live-Tested In This Pass

- Real Colab REPL execution.
- Real `/colab/tty` shell.
- Real Drive OAuth/credential propagation.
- Real kernel restart/interrupt.
- Real OAuth code exchange.

Run only with explicit credentials/quota approval:

```text
COLAB_CLI_LIVE=1 ./scripts/live-smoke.sh
COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 ./scripts/live-secrets-smoke.sh
```

## Release Gate Still Required

Standard release gate:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo doc --workspace --all-features --no-deps
```
