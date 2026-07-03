# Publishing

Dry-run only:

```sh
cargo package --allow-dirty
cargo publish --dry-run -p colab-cli
```

Do not run real `cargo publish` without an explicit maintainer instruction.

Release checklist:

- `cargo fmt --all --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `cargo doc --no-deps`
- `cargo package --allow-dirty`
- `cargo publish --dry-run -p colab-cli`
- verify no credentials are included
- tag only after dry-runs pass

Only `colab-cli` is published. Internal `src/cocli/*` modules are not separate crates.
