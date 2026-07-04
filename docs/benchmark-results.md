# Benchmark Results

## Environment

| Field | Value |
|---|---|
| Date | 2026-07-04 |
| OS | Darwin arm64 |
| Rust | `rustc 1.91.1`, `cargo 1.91.1` |
| Build profile | release |
| Network | unavailable for deterministic benchmark runs |
| Timing tool | `hyperfine` available |

## Competitors And Versions

| Tool | Version/commit | Install method | Status |
|---|---|---|---|
| google-colab-cli | local checkout under `google-colab-cli/` | source checkout | not executed; Python deps not installed |
| colab-mcp | GitHub reference | docs only | not benchmarked |
| release-plz | not in PATH | n/a | not measured |
| cargo-release | not in PATH | n/a | not measured |
| git-cliff | not in PATH | n/a | not measured |
| release-please | not in PATH | n/a | not measured |

## Results

Run local benchmarks with:

```sh
./scripts/bench-colab.sh
../shipyard/scripts/bench-shipyard.sh
```

| Scenario | Tool | Time | Memory | Commands | Bytes | Output lines | Errors | Notes |
|---|---|---:|---:|---:|---:|---:|---:|---|
| startup help | colab | 1.38 ms median | n/a | 1 | n/a | help text | 0 | local release binary, hyperfine |
| status quick | colab | 1.54 ms median | n/a | 1 | n/a | compact JSON | 0 | local release binary, hyperfine |
| fs sync dry-run | colab | 1.64 ms median | n/a | 1 | local only | JSON only | 0 | remote manifest cache not implemented |
| release name | Shipyard | 2.54 ms median | n/a | 1 | n/a | 1 | 0 | local release binary, hyperfine |
| release bench | Shipyard | 173.58 ms median | n/a | 1 | n/a | short | 0 | release-plz not installed |
| release plan --why | Shipyard | 103.24 ms median | n/a | 1 | n/a | short | 0 | release-plz not installed |

## Gain Table

| Scenario | Metric | Baseline | New | Gain | Claim |
|---|---|---:|---:|---:|---|
| colab no-op sync | bytes sent | not measured | not measured | n/a | not proven |
| Shipyard plan --why | wall time | release-plz not measured | 103.24 ms | n/a | not proven |
| Shipyard bench | wall time | release-plz not measured | 173.58 ms | n/a | not proven |
| Shipyard binary | bytes | release-plz not measured | 1,835,520 prior sample | n/a | not proven |

## Claims Accepted

None yet.

## Claims Rejected

- `10x faster startup`: not measured against a competitor.
- `50x less upload work`: remote unchanged-tree baseline is not implemented in local dry-run.
- `better UX`: no user study has been run.

## Next Optimization Targets

- colab: cache remote manifests so no-op sync can prove unchanged-tree savings.
- colab: run google-colab-cli comparison after Python deps are installed.
- Shipyard: install release-plz, cargo-release, and git-cliff in a pinned tool cache and rerun comparison.
