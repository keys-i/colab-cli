# Build Breakers

Date: 2026-07-04

## Baseline Commands

Run before transport/UI changes:

```text
cargo check --workspace --all-features
cargo test --workspace --all-features
```

Both passed.

## Findings

| Area | Finding | Status |
|---|---|---|
| Missing handlers | The previously reported `handle_secret` compile failure is not present in the current tree. | clear |
| Binary naming | Package remains `colab-cli`; primary binary is `colab`; compatibility binary is `colab-cli`. | clear |
| Stale target binary | `target/debug/colab` can be stale after `cargo check`; smoke output must use `cargo run --bin colab -- ...` or rebuild first. | documented |
| Hidden agent alias | Old `agent` alias could still execute plan code. | fixed |
| AI subhelp leaks | `ai --help` showed hidden/gated `mcp`, `plan`, `run`, and `code` surfaces. | fixed |
| Runtime session path | REPL/Drive/kernel current used tunnel `/api/sessions` before the runtime proxy path. | fixed |

## Verification Added

- `ai_help_hides_gated_execution_surfaces`
- `old_agent_alias_does_not_execute_plan`
- existing all-features CLI and unit tests

## Current Gate

No public command route may point to a missing handler. Hidden aliases may parse
only for migration and must not bypass experiment gates.
