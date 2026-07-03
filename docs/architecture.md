# Architecture

`colab-cli` is one publishable Rust package.

Implementation lives under `src/cocli/`, grouped by command space. The public API is intentionally tiny: the binary calls `cocli::cli::dispatch`, and most modules are internal implementation detail.

Why one crate:

- the public API is not stable yet
- command handlers share auth, session, and UI code
- separate crates added versioning and publish work without real reuse
- future crate extraction is still possible when an external user proves the boundary

Current layout:

```text
src/main.rs
src/lib.rs
src/cocli/
  cli/        clap args and dispatch
  auth/       OAuth, token storage, profiles, redaction
  session/    Colab client, session model, local session store
  exec/       shell and command runners
  fs/         manifests, excludes, sync planning
  runtime/    runtime metadata and Colab command snippets
  slurp/      Slurp TOML parsing and explanation
  fleet/      compliance checks and scheduler planning
  continue/   continuation manifest and resume planning
  tools/      built-in registry shown through settings skills
  config/     config files behind settings
  release/    release names and notes helpers
  ui/         terminal output
  util/       ids, paths, time, JSON helpers
```

Invariants:

- no unsafe code
- JSON output contains no ANSI
- destructive commands need explicit confirmation
- continuation is checkpoint/replay, not live Python memory transfer
- fleet mode is planning for approved runtimes, not free-tier quota bypass
