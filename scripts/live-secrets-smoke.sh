#!/usr/bin/env bash
set -euo pipefail

if [[ "${COLAB_CLI_LIVE:-}" != "1" || "${COLAB_CLI_SECRET_TEST:-}" != "1" ]]; then
  echo "set COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 to run live secrets smoke" >&2
  exit 2
fi

if [[ -z "${COCLI_TEST_SECRET:-}" ]]; then
  echo "set COCLI_TEST_SECRET to a non-production test value" >&2
  exit 2
fi

BIN="${COLAB_BIN:-colab}"

"$BIN" settings experiments set secrets-bridge true >/dev/null

out="$("$BIN" run py --env COCLI_TEST_SECRET --code "from google.colab import userdata; v=userdata.get('COCLI_TEST_SECRET'); print('secret_len', len(v))")"

if grep -Fq "$COCLI_TEST_SECRET" <<<"$out"; then
  echo "secret value leaked in live smoke output" >&2
  exit 1
fi

grep -q "secret_len" <<<"$out"
echo "$out"
