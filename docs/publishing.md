# Publishing

Dry-run only:

```sh
cargo package --workspace --allow-dirty
cargo publish --dry-run -p cocli-protocol
cargo publish --dry-run -p cocli-core
cargo publish --dry-run -p cocli-colab
cargo publish --dry-run -p cocli-fs
cargo publish --dry-run -p cocli-tools
cargo publish --dry-run -p colab-cli
```

Do not run real `cargo publish` without an explicit maintainer instruction.

Release checklist:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- package dry-run for every crate
- verify crate README files render
- verify no credentials are included
- tag only after dry-runs pass

Crate metadata is inherited from the workspace where possible. Each crate still has its own description and README.
