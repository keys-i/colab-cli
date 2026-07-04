# Performance Pass

Measured with `hyperfine --warmup 3 --runs 10` against `target/debug/colab` on this checkout.

| Command | Before | After | Gain | Notes |
|---|---:|---:|---:|---|
| `colab --help` | not measured | 4.9 ms ± 0.2 ms | not claimed | fixed help template, no config/auth load |
| `colab status` | not measured | 6.7 ms ± 2.2 ms | not claimed | local checks only; outliers observed |
| `colab settings` | not measured | 4.9 ms ± 0.4 ms | not claimed | one config load |
| `colab ai tools list` | not measured | 9.2 ms ± 0.4 ms | not claimed | catalog loads config to hide gated tools |
| `colab run pip --help` | not measured | 4.8 ms ± 0.3 ms | not claimed | clap help only |
| `colab run ast examples/sample.py` | not measured | 5.3 ms ± 0.2 ms | not claimed | local parser, AST experiment enabled |
| `colab fs sync src /content/src --dry-run` | not measured | 6.0 ms ± 0.3 ms | not claimed | local dry-run path |

No 5x, 10x, or 100x claim is made here. A gain needs a measured baseline from the same checkout and build profile.
