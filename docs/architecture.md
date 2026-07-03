# Architecture

The workspace has six publishable crates.

`colab-cli` owns the binary, auth flow, Colab HTTP client, session manager, terminal bridge, and command handlers. Existing working Colab code stayed here to avoid a risky move across crate boundaries.

`cocli-core` owns small local utilities: config, color policy, terminal bell policy, duration parsing, and compact session lookup.

`cocli-colab` owns safe Colab-facing request intent and command snippets. It does not copy `google.colab` Python implementation code.

`cocli-protocol` owns JSON structs for continuation manifests, file entries, execution steps, and tool specs.

`cocli-fs` owns local file manifests, default excludes, sync diff planning, safe local joins, and remote chunk plans.

`cocli-tools` owns the built-in tool registry. It uses enum dispatch instead of an async trait until external plugin execution needs dynamic dispatch.

Invariants:

- no unsafe code
- user errors return typed errors, not panics
- destructive new commands require explicit confirmation
- JSON output contains no ANSI codes
- terminal bell is opt-in and disabled in CI or quiet mode
