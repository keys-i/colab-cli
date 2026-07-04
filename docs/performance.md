# Performance

Budgets:

- `colab --help` under 80 ms locally where realistic
- normal config load under 5 ms
- manifest diff handles 10k files without a full hash pass
- chunk planning does not read file bytes
- no full file hashing unless the caller opts in

Benchmarks:

```sh
cargo bench
```

Coverage:

- config load
- compact session lookup
- file manifest diff
- local manifest build without hashing
- continuation manifest serialize and deserialize
- command parse smoke path
- remote file chunk planner

Optional local tools:

```sh
cargo flamegraph -- --help
cargo bloat --release
```

Benchmark numbers are not CI gates.
