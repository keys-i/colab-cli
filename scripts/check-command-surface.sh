#!/usr/bin/env sh
set -eu

cargo run --bin colab -- --help >/tmp/colab-help.txt
cargo run --bin colab -- status --help >/tmp/colab-status-help.txt
cargo run --bin colab -- run --help >/tmp/colab-run-help.txt
cargo run --bin colab -- fs --help >/tmp/colab-fs-help.txt
cargo run --bin colab -- settings skills list --help >/tmp/colab-skills-help.txt

for old in exec env mount runtime tools config doctor release agent; do
  if grep -q "  $old" /tmp/colab-help.txt; then
    echo "old top-level command leaked into help: $old" >&2
    exit 1
  fi
done

if rg -n "colab (exec|env|mount|runtime|tools|config|doctor|drivemount)\\b" README.md docs \
  -g '!docs/audit/**' \
  -g '!docs/command-audit.md' \
  -g '!docs/google-colab-cli-map.md' \
  -g '!docs/migration-from-google-colab-cli.md'
then
  echo "old command example leaked outside migration docs" >&2
  exit 1
fi
