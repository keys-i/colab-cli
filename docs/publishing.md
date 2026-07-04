# Publishing

Shipyard is the release front door:

```sh
cargo shipyard plan
cargo shipyard update --dry-run
cargo shipyard release --dry-run
```

Cargo remains the publisher. Shipyard delegates package upload and dry-run checks to `cargo publish`.

Manual Cargo dry-run:

```sh
cargo package --allow-dirty
cargo publish --dry-run -p colab
```

Do not run real `cargo publish` without an explicit maintainer instruction.

Release checklist:

- `cargo fmt --all --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `cargo doc --no-deps`
- `cargo package --allow-dirty`
- `cargo publish --dry-run -p colab`
- `cargo shipyard compare-release-plz --repo .` when comparing release tooling
- verify no credentials are included
- tag only after dry-runs pass

Only the Cargo package `colab-cli` is published. The primary installed binary is `colab`; `colab-cli` is a compatibility alias. Internal `src/cocli/*` modules are not separate crates.
