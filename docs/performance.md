# Performance

Budgets:

- `colab-cli --help` under 80 ms locally where realistic
- normal config load under 5 ms
- manifest diff handles 10k files without a full hash pass
- chunk planning does not read file bytes
- no full file hashing unless the caller opts in

Benchmarks:

```sh
cargo bench --workspace
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
cargo flamegraph -p colab-cli -- --help
cargo bloat --release -p colab-cli
```

Benchmark numbers are not CI gates.
