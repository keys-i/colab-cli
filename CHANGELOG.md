# Changelog

## 0.2.0

- Added Cargo workspace crates: `colab-cli`, `cocli-core`, `cocli-colab`, `cocli-tools`, `cocli-protocol`, `cocli-fs`.
- Added grouped command space: `session`, `exec`, `fs`, `mount`, `env`, `runtime`, `tools`, `agent`, `continue`, `config`, `doctor`.
- Kept hidden compatibility groups and cheap aliases for existing workflows.
- Added continuation manifests with checkpoint and replay semantics.
- Added file manifest diffing, default excludes, and chunk planning.
- Added built-in tool registry and JSON plan output.
- Removed the only unsafe block.
- Deferred `fs sync --watch`, `fs edit`, and transparent local `exec -f` parity until live Colab behavior is verified.
