# CI/CD

## Pull Requests

`ci.yml` runs:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --verbose`
- `cargo test --all-targets --verbose`
- `cargo doc --no-deps`
- `cargo package --allow-dirty`

PR jobs use `contents: read` and do not receive release secrets.

## Main

Pushes to `main` run the same CI jobs. Release planning is handled separately by Shipyard.

## Scheduled Releases

`shipyard-release.yml` runs at `17 3 1,16 * *` UTC. That is close to every 15 days and easy to audit.

If there are no releasable changes, Shipyard exits with a no-op message instead of publishing an empty release.

## Publishing

Publishing runs only for `colab-v*.*.*` tags.

Required secret:

- `CARGO_REGISTRY_TOKEN`

No Google or Colab credentials are used in CI.

## Permissions

CI uses read-only contents access. The Shipyard release PR job uses `contents: write` and `pull-requests: write`. The publish job uses `contents: write` and `id-token: write`.

No workflow uses `write-all`.
