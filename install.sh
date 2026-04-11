#!/bin/sh
# colab-cli installer
#
#   curl -fsSL https://raw.githubusercontent.com/keys-i/colab-cli/main/install.sh | sh
#
# Installs `colab-cli` from crates.io. If `cargo` is missing, bootstraps the
# Rust toolchain via rustup first.

set -eu

CRATE="colab-cli"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
err() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1; }

ensure_cargo() {
    if need cargo; then
        return
    fi

    say "cargo not found — installing rustup (default toolchain: stable)"
    if ! need curl; then
        err "curl is required to bootstrap rustup"
    fi
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal

    # shellcheck disable=SC1090
    . "${CARGO_HOME:-$HOME/.cargo}/env"

    if ! need cargo; then
        err "rustup install completed but cargo is still not on PATH — open a new shell and re-run this script"
    fi
}

main() {
    ensure_cargo

    say "installing $CRATE from crates.io"
    cargo install "$CRATE" --locked

    if need "$CRATE"; then
        say "installed: $($CRATE --version 2>/dev/null || echo "$CRATE")"
        say "run \`$CRATE auth login\` to get started"
    else
        cat >&2 <<EOF
$CRATE was installed but is not on your PATH.
Add this to your shell profile:

    export PATH="\$HOME/.cargo/bin:\$PATH"

EOF
        exit 1
    fi
}

main "$@"
