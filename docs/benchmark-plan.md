# Benchmark Plan

Benchmarks are split into deterministic local checks and noisy network/API checks.

Rules:

- release binaries only
- at least 10 iterations for local timing
- median and p90 when `hyperfine` is available
- no debug-build comparisons
- competitor versions recorded before comparison
- failed or missing competitor runs stay in the table
- no network/API benchmark can create a public claim by itself

## colab Scenarios

| Scenario | Baseline | Command | Metric |
|---|---|---|---|
| Startup | google-colab-cli help if installed | `colab --help` | wall time |
| Status quick | manual diagnosis | `colab --json status quick` | time, output lines |
| File sync dry-run | manual upload/download | `colab --json fs sync DIR /content/dir --dry-run` | time, JSON cleanliness |
| Manifest diff 10k | naive full scan | `cargo bench --bench manifest` | diff time |
| Command parse | clap parser path | `cargo bench --bench hot_paths command_parse_smoke` | parse time |

Remote Colab scenarios require credentials and are marked noisy.

## Shipyard Scenarios

| Scenario | Baseline | Command | Metric |
|---|---|---|---|
| Startup | release-plz/cargo-release if installed | `cargo-shipyard animal --version 0.4.2` | wall time |
| Plan | release-plz update dry-run if installed | `cargo-shipyard bench --repo PATH` | plan ms |
| Changelog | git-cliff if installed | `cargo-shipyard notes --human` | time, quality checklist |
| Safety | manual release checklist | `cargo-shipyard safety` | output lines, gates shown |

Large workspaces are generated only in local scratch directories.
