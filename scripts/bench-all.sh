#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/scripts/bench-cocli.sh"

if [ -x "$ROOT/../shipyard/scripts/bench-shipyard.sh" ]; then
  "$ROOT/../shipyard/scripts/bench-shipyard.sh"
else
  echo "shipyard bench script not found at ../shipyard/scripts/bench-shipyard.sh"
fi
