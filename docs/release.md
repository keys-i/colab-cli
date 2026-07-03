# Release

This repo uses Shipyard for release planning.

Shipyard replaced release-plz here because it is local, small, and tailored to this single-package crate. The broader claim that it is faster or smaller than release-plz is not proven yet unless `cargo shipyard compare-release-plz --repo .` has measured both tools on the same machine.

## Local Flow

```sh
cargo shipyard plan
cargo shipyard update --dry-run
cargo shipyard update
cargo shipyard pr --base main
cargo shipyard release --dry-run
```

Real publishing is explicit:

```sh
cargo shipyard release --yes
```

Do not run that unless the release PR has passed CI and the maintainer intends to publish.

## Scheduled Flow

`.github/workflows/shipyard-release.yml` runs on the 1st and 16th of each month at 03:17 UTC, plus manual dispatch.

The release PR job:

- installs Shipyard from a pinned Git tag
- runs `cargo shipyard plan`
- runs `cargo shipyard update`
- opens or updates the release PR

The publish job runs only for `colab-cli-v*.*.*` tags. It is the only job that receives `CARGO_REGISTRY_TOKEN`.

## Install Source

Before Shipyard is published to crates.io, CI installs it from:

```text
https://github.com/keys-i/shipyard tag cargo-shipyard-v0.1.0
```

After Shipyard is published, the install step can become:

```sh
cargo install cargo-shipyard --locked
```

The workflow never relies on `../shipyard`.

## Comparison

Local comparison output is written to:

```text
target/shipyard/comparison-release-plz.md
```

Current local result: release-plz was not installed, so release-plz timings are not measured. Shipyard measured the colab-cli plan path locally, but no superiority claim is made from one-sided data.
