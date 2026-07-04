# Performance Pass

Measured with `hyperfine --warmup 3 --runs 10` against `target/debug/colab-cli` on this checkout.

| Command | Before | After | Gain | Notes |
|---|---:|---:|---:|---|
| `colab-cli --help` | not measured | 4.5 ms ± 0.2 ms | not claimed | fixed help template, no config/auth load |
| `colab-cli status` | not measured | 5.2 ms ± 0.2 ms | not claimed | local checks only |
| `colab-cli settings` | not measured | 5.1 ms ± 0.3 ms | not claimed | one config load |
| `colab-cli settings skills list` | not measured | 5.2 ms ± 0.3 ms | not claimed | fixed catalog, no network/runtime init |
| `colab-cli fs sync /private/tmp/cocli-perf /content/tmp --dry-run` | not measured | 5.5 ms ± 0.3 ms | not claimed | one-file local manifest |
| `colab-cli status check` | not measured | 5.1 ms ± 0.2 ms | not claimed | same local report as status |

These are current local measurements, not competitor claims.
