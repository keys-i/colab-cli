# Claims Ledger

| Claim | Project | Metric | Baseline | Result | Ratio | Status |
|---|---|---:|---:|---:|---:|---|
| 10x faster startup | colab | startup ms | not measured | not measured | n/a | not proven |
| 5x faster help | colab | help ms | not measured | 4.9 ms ± 0.2 ms | n/a | not proven |
| 50x less upload work | colab | bytes sent | not measured | not measured | n/a | not proven |
| fewer first-run commands | colab | command count | not measured | not measured | n/a | not proven |
| JSON output has zero ANSI | colab | ANSI escape count | 0 target | 0 in tests | n/a | proven |
| no raw JSON in default status | colab | human status output | no raw JSON target | covered by tests | n/a | proven |
| no secrets printed in redaction tests | colab | leaked token substrings | 0 target | 0 in tests | n/a | proven |
| 10x faster release plan | Shipyard | plan ms | release-plz not measured | not measured in this pass | n/a | not proven |

README performance claims must stay out until a row is `proven` with a measured baseline.
