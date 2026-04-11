# colab-cli

> Google Colab from your terminal — powered by Rust.

`colab-cli` brings Google Colab out of the browser and into your shell. Assign a
runtime, drop into an interactive shell, stream files, watch live GPU stats,
and run arbitrary commands — all without ever opening a notebook tab.

- **Fast.** Native Rust binary, rustls (no OpenSSL), LTO release builds.
- **Interactive.** Real PTY shell over WebSockets, live `top`-style stats.
- **Scriptable.** Clean subcommand surface, passthrough `run`/`ls`/`cp`/`rm`.
- **Persistent.** Optional keepalive pings and automatic OAuth token refresh.

---

## Table of contents

- [Features](#features)
- [Quick start](#quick-start)
- [Usage](#usage)
- [Configuration](#configuration)
- [Build from source](#build-from-source)
- [Shell completions](#shell-completions)
- [Troubleshooting](#troubleshooting)
- [License](#license)

---

## Features

| Area | What you get |
| --- | --- |
| **Auth** | Browser-based Google OAuth, securely cached credentials, auto-refresh |
| **Servers** | Assign / reconfigure / list / remove CPU, GPU, and TPU runtimes |
| **Shell** | Full interactive PTY shell over WebSockets (`colab-cli server shell`) |
| **Run** | Stream stdout/stderr from arbitrary remote commands with real exit codes |
| **Files** | Upload local files, plus passthrough `ls` / `cp` / `rm` on the runtime |
| **Monitor** | Realtime CPU / RAM / disk / GPU stats via `colab-cli server ps` |
| **Keepalive** | `-k` flag on `assign` / `reconfigure` keeps the runtime warm indefinitely |
| **Completions** | First-class bash / zsh / fish / PowerShell / elvish completions |

---

## Quick start

### 1. Install

**One-liner (macOS / Linux):**

```sh
curl -fsSL https://raw.githubusercontent.com/keys-i/colab-cli/main/install.sh | sh
```

The script installs `cargo` via rustup if needed, then runs
`cargo install colab-cli --locked`. The crate published to crates.io has OAuth
credentials baked in at build time, so the resulting binary works out of the
box — no `.env`, no `config.toml`, nothing to set up.

**Manual (any platform with Rust):**

```sh
cargo install colab-cli --locked
```

**Building from a git checkout** is a different story — you'll need to provide
your own OAuth client. See [Build from source](#build-from-source).

### 2. Sign in

```bash
colab-cli auth login
```

Your browser opens, you approve the scopes, and credentials are cached in your
OS data directory.

### 3. Assign a runtime and jump in

```bash
# Interactive picker (variant + accelerator)
colab-cli server assign

# Or non-interactively:
colab-cli server assign --variant gpu --accelerator T4 --high-ram -k

# Drop into a real shell on the runtime
colab-cli server shell
```

That's it — you're in Colab, from your terminal.

---

## Usage

All commands are available under `colab-cli <group> <command>`. Run any
subcommand with `--help` for full flag docs.

### Auth

```bash
colab-cli auth login        # sign in via browser
colab-cli auth logout       # clear stored credentials
```

### Servers

```bash
colab-cli server assign [--variant cpu|gpu|tpu] [-a T4] [--high-ram] [-k]
colab-cli server reconfigure [--name NAME] [--variant ...] [-a ...] [--high-ram] [-k]
colab-cli server ls                 # list assigned servers
colab-cli server ls --available     # show available accelerators + CCU/hr rates
colab-cli server info [--name NAME] # server + account details
colab-cli server ps [--interval 1000]   # live CPU / RAM / disk / GPU stats
colab-cli server shell [--name NAME]    # interactive PTY shell
colab-cli server run  [--name NAME] -- python -V
colab-cli server rm   [--name NAME]
```

Examples:

```bash
colab-cli server run --name "Colab GPU" nvidia-smi
colab-cli server run -- bash -lc 'pip install torch && python train.py'
```

### Files

```bash
colab-cli file upload ./dataset.csv /content/dataset.csv
colab-cli file ls                        # defaults to `ls -lah /content`
colab-cli file ls -- -lah /tmp
colab-cli file cp -- -r /content/a /content/b
colab-cli file rm -- -rf /content/junk
```

Anything after `--` is forwarded verbatim to the remote `ls` / `cp` / `rm`.

---

## Configuration

`colab-cli` resolves OAuth credentials in this precedence order:

1. **Environment variables** (including a `.env` file in the current directory)
2. **`~/.config/colab-cli/config.toml`**
3. **Build-time embedded values** — release binaries have OAuth credentials
   baked in at compile time via `build.rs` and obfuscated with
   [`obfstr`](https://crates.io/crates/obfstr), so end users of release
   builds don't need to configure anything

Non-auth settings (`COLAB_EXTENSION_ENVIRONMENT`, `COLAB_DOMAIN`,
`COLAB_QUIET`) still come from the environment or `config.toml`.

### Environment variables

| Variable | Purpose | Required? |
| --- | --- | --- |
| `COLAB_EXTENSION_CLIENT_ID` | OAuth client ID | Only when building from source |
| `COLAB_EXTENSION_CLIENT_NOT_SO_SECRET` | OAuth client secret | Only when building from source |
| `COLAB_EXTENSION_ENVIRONMENT` | `production`, `sandbox`, or `local` | No — defaults to `production` |
| `COLAB_DOMAIN` | Override the Colab base URL | No |
| `COLAB_QUIET` | Suppress non-essential output | No |

### `config.toml` example

```toml
# ~/.config/colab-cli/config.toml
environment  = "production"
# Only needed for source builds that didn't bake credentials in at build time.
# client_id    = "your-oauth-client-id.apps.googleusercontent.com"
# client_secret = "your-oauth-client-secret"
# colab_domain = "https://colab.research.google.com"
```

### File locations

- **Config:** `~/.config/colab-cli/config.toml` (or your platform's config dir)
- **Data:** `~/.local/share/colab-cli/` — cached credentials and
  `servers.json` live here

---

## Build from source

### Prerequisites

- **Rust 1.85+** (edition 2024) — install via [rustup.rs](https://rustup.rs)
- A working C toolchain (for transitive build scripts)
- No OpenSSL needed — TLS is handled by rustls
- A Google OAuth 2.0 client (Desktop application type) if you want your
  binary to authenticate — this is only required for local source builds;
  official release binaries ship with credentials already baked in

### Provide OAuth credentials (source builds only)

Export the two variables below before `cargo build`, or drop them in a
`.env` file at the repo root, or in `~/.config/colab-cli/config.toml`:

```bash
export COLAB_EXTENSION_CLIENT_ID="your-oauth-client-id.apps.googleusercontent.com"
export COLAB_EXTENSION_CLIENT_NOT_SO_SECRET="your-oauth-client-secret"
```

When set at build time, [`build.rs`](build.rs) bakes the values into the
binary via `obfstr` so they don't appear in plaintext in the final executable.
When unset, you can still provide them at runtime through the same env vars
or `config.toml`.

### Clone and build

```bash
git clone https://github.com/keys-i/colab-cli
cd colab-cli

# Debug build
cargo build

# Optimized release build (LTO, strip, panic=abort)
cargo build --release
```

The release binary lands at `target/release/colab-cli`.

### Install locally

```bash
cargo install --path . --locked
```

This compiles a release build and installs `colab-cli` into `~/.cargo/bin`.

### Run the test suite

```bash
cargo test
```

### Run the benchmarks

```bash
cargo bench
```

Criterion HTML reports are written to `target/criterion/`.

### Development loop

```bash
cargo run -- server ls          # run against your working tree
cargo clippy --all-targets      # lints
cargo fmt                       # format
```

> Note: `build.rs` writes a generated `src/embedded.rs` containing the
> obfuscated OAuth credentials. The file is gitignored. CI populates it via
> the `COLAB_EXTENSION_CLIENT_ID` / `COLAB_EXTENSION_CLIENT_NOT_SO_SECRET`
> repo secrets before `cargo publish`, and `package.include` forces the
> populated file into the published `.crate` so end users get a working
> binary out of `cargo install`.

---

## Shell completions

Generate completions for your shell and source them:

```bash
# Bash
colab-cli completions bash > ~/.local/share/bash-completion/completions/colab-cli

# Zsh (anywhere on $fpath)
colab-cli completions zsh > ~/.zfunc/_colab-cli

# Fish
colab-cli completions fish > ~/.config/fish/completions/colab-cli.fish

# PowerShell
colab-cli completions powershell | Out-String | Invoke-Expression
```

---

## Troubleshooting

**`COLAB_EXTENSION_CLIENT_ID is not set`**
You're running a binary built from a fresh git checkout without your own OAuth
client. Either install via `cargo install colab-cli --locked` (credentials are
baked into the published crate), or follow [Build from source](#build-from-source).

**`colab-cli auth login` hangs / browser doesn't open**
The OAuth flow spins up a local loopback listener. Make sure nothing else is
bound to the loopback port it prints, and that your firewall allows local
connections.

**Runtime disconnects after a few minutes of idle**
Colab aggressively recycles idle runtimes. Pass `-k` / `--keepalive` to
`assign` or `reconfigure` to keep the session warm via periodic pings and
automatic token refresh.

**`colab-cli server shell` renders garbled output**
Your terminal's `TERM` is probably unusual. Try `TERM=xterm-256color colab-cli server shell`.

---

## License

Licensed under the [MIT License](LICENSE). © 2026 keys-i.
