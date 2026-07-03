#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/bench"
FIXTURE="$OUT/cocli-fixture"
BIN="$ROOT/target/release/colab-cli"
mkdir -p "$OUT" "$FIXTURE"
printf 'ok\n' > "$FIXTURE/a.txt"

cargo build --release --manifest-path "$ROOT/Cargo.toml" >/dev/null

if command -v hyperfine >/dev/null 2>&1; then
  hyperfine --warmup 2 --runs 10 --export-json "$OUT/cocli-hyperfine.json" \
    "$BIN --help" \
    "$BIN --json status quick" \
    "$BIN --json fs sync '$FIXTURE' /content/tmp --dry-run" >/dev/null
else
  "$BIN" --json status quick > "$OUT/cocli-status.json"
fi

cat > "$OUT/cocli-benchmark.md" <<EOF
# cocli Benchmark

Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
Binary: $BIN

Local deterministic scenarios:

- startup help
- JSON status quick
- JSON fs sync dry-run against a tiny local tree

Competitor commands were not run by this script unless installed and configured locally.
EOF

echo "wrote $OUT/cocli-benchmark.md"
