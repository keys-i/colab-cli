# Refactor Map

| Old path | New path | Notes |
|---|---|---|
| `crates/colab-cli/src/main.rs` | `src/cocli/cli/dispatch.rs` | Binary logic moved behind a small `src/main.rs` shim. |
| `crates/colab-cli/src/cli.rs` | `src/cocli/cli/args.rs` | Clap command definitions. |
| `crates/colab-cli/src/auth/*` | `src/cocli/auth/*` | OAuth and token storage stayed together. |
| `crates/colab-cli/src/client/*` | `src/cocli/session/client.rs`, `src/cocli/session/model.rs` | Colab HTTP client and response models. |
| `crates/colab-cli/src/server/*` | `src/cocli/session/commands.rs`, `src/cocli/session/store.rs` | Existing session manager and local store. |
| `crates/colab-cli/src/shell.rs` | `src/cocli/exec/runner.rs` | Remote shell and streaming execution. |
| `crates/colab-cli/src/config.rs` | `src/cocli/config/file.rs` | Colab config plus CLI UI config. |
| `crates/cocli-core/src/auth_profiles.rs` | `src/cocli/auth/profiles.rs` | Multi-profile metadata. |
| `crates/cocli-core/src/slurp.rs` | `src/cocli/slurp/config.rs` | Slurp TOML parsing and explanation. |
| `crates/cocli-core/src/compliance.rs` | `src/cocli/fleet/compliance.rs` | Fleet policy checks. |
| `crates/cocli-core/src/scheduler.rs` | `src/cocli/fleet/scheduler.rs` | Simple fleet planner. |
| `crates/cocli-core/src/release_names.rs` | `src/cocli/release/names.rs` | Deterministic release naming. |
| `crates/cocli-core/src/rng.rs` | `src/cocli/util/ids.rs` | Secret/public ID helpers. |
| `crates/cocli-fs/src/lib.rs` | `src/cocli/fs/manifest.rs` | File manifest, diff, chunk planning. |
| `crates/cocli-protocol/src/lib.rs` | `src/cocli/continue/manifest.rs` | Continuation manifest and JSON protocol structs. |
| `crates/cocli-tools/src/lib.rs` | `src/cocli/tools/registry.rs` | Small built-in registry. |
| `crates/cocli-colab/src/lib.rs` | `src/cocli/runtime/info.rs` | Runtime helper snippets and URLs. |
| `crates/*` | removed | No proven public API boundary yet. |
