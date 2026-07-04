#!/usr/bin/env bash
set -u

if [ "${COLAB_CLI_LIVE:-${COLAB_CLI_LIVE:-}}" != "1" ]; then
  echo "set COLAB_CLI_LIVE=1 to run live Colab smoke tests"
  exit 2
fi

mkdir -p target
report="target/live-smoke.md"
bin=(cargo run --quiet --)
session_arg=(--session -)
failures=0

{
  echo "# Live Smoke"
  echo
  echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo
} >"$report"

run_step() {
  local name="$1"
  shift
  echo "==> $name"
  {
    echo "## $name"
    echo
    echo '```text'
    printf '$'
    printf ' %q' "$@"
    echo
  } >>"$report"
  if "$@" >>"$report" 2>&1; then
    echo '```' >>"$report"
    echo "ok"
  else
    echo '```' >>"$report"
    echo "failed"
    failures=$((failures + 1))
  fi
  echo >>"$report"
}

skip_step() {
  local name="$1"
  local reason="$2"
  echo "==> $name: skipped ($reason)"
  {
    echo "## $name"
    echo
    echo "skipped: $reason"
    echo
  } >>"$report"
}

run_step "session list" "${bin[@]}" session list
run_step "run python" "${bin[@]}" run py "${session_arg[@]}" --code "print('colab-live-ok')"
run_step "runtime status" "${bin[@]}" status runtime --all
run_step "fs ls content" "${bin[@]}" fs ls /content "${session_arg[@]}"
run_step "drive status before mount" "${bin[@]}" fs drive status "${session_arg[@]}"

if [ -t 0 ]; then
  echo "Drive may need browser approval."
  echo "Opening the session URL before mount."
  "${bin[@]}" session url "${session_arg[@]}" --open || true
  echo "Approve Drive in the browser if Colab asks, then press Enter."
  read -r _
  run_step "drive mount" "${bin[@]}" fs drive mount "${session_arg[@]}" --timeout 120
else
  skip_step "drive mount" "browser approval may be required in a non-interactive shell"
fi

run_step "drive status after mount" "${bin[@]}" fs drive status "${session_arg[@]}"
run_step "skills list" "${bin[@]}" settings skills list
run_step "skills list json" "${bin[@]}" --json settings skills list
run_step "status check" "${bin[@]}" status check

{
  echo "## Summary"
  echo
  echo "failures: $failures"
} >>"$report"

echo "report: $report"
exit "$failures"
