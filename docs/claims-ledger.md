# Claims Ledger

| Claim | Project | Metric | Baseline | Result | Ratio | Status |
|---|---|---:|---:|---:|---:|---|
| 10x faster startup | cocli | cold start ms | not measured | not measured | n/a | not proven |
| 50x less upload work | cocli | bytes sent | not measured | not measured | n/a | not proven |
| 2x fewer first-run commands | cocli | command count | not measured | not measured | n/a | not proven |
| local startup under 5 ms | cocli | median startup ms | 5 target | 1.38 | n/a | proven local target |
| local status quick under 5 ms | cocli | median runtime ms | 5 target | 1.54 | n/a | proven local target |
| JSON output has zero ANSI | cocli | ANSI escape count | 0 target | 0 in tests | n/a | proven |
| zero token leaks in redaction tests | cocli | leaked token substrings | 0 target | 0 in tests | n/a | proven |
| 10x faster release plan | Shipyard | plan ms | release-plz not measured | 103.24 | n/a | not proven |
| local release-name startup under 5 ms | Shipyard | median startup ms | 5 target | 2.54 | n/a | proven local target |
| smaller release binary | Shipyard | bytes | release-plz not measured | 1,835,520 sample | n/a | not proven |
| JSON output has zero ANSI | Shipyard | ANSI escape count | 0 target | 0 in tests | n/a | proven |
| no secrets printed in tests | Shipyard | leaked token substrings | 0 target | 0 in tests | n/a | proven |

README performance claims must stay out until a row is `proven`.
