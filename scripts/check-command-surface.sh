#!/usr/bin/env sh
set -eu

cargo run -- --help >/tmp/cocli-help.txt
cargo run -- status --help >/tmp/cocli-status-help.txt
cargo run -- run --help >/tmp/cocli-run-help.txt
cargo run -- fs --help >/tmp/cocli-fs-help.txt
cargo run -- settings skills list --help >/tmp/cocli-skills-help.txt

for old in exec env mount runtime tools config doctor; do
  if grep -q "  $old" /tmp/cocli-help.txt; then
    echo "old top-level command leaked into help: $old" >&2
    exit 1
  fi
done

if rg -n "colab-cli (exec|env|mount|runtime|tools|config|doctor|drivemount)" README.md docs \
  -g '!docs/command-audit.md' -g '!docs/migration-from-google-colab-cli.md'
then
  echo "old command example leaked outside migration docs" >&2
  exit 1
fi
