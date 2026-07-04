# AI Tools Audit

Scope: `ai tools`, `settings skills`, built-in tool registry, AI docs, and gating tests.

## Findings

- `ai tools list` is read-only and available by default. The rendered catalog shows agent-facing workflows, not core command spam.
- The catalog includes workflow, distribute, runtime, file, support, MCP, AST, agent, and kernel rows.
- Default states are clear: distribute rows are `gated`, MCP rows are `off`, AST rows are `off`, and ready rows do not imply hidden execution.
- JSON output exposes stable fields: `name`, `scope`, `category`, `risk`, `needs_session`, `network`, `state`, `summary`, `inputs`, `outputs`, `examples`, `safety_notes`, and `json_schema`.
- `ai mcp`, `ai plan`, and `ai run` are experiment-gated. `ai run` also requires explicit confirmation.
- Built-in tool planning is enum-driven. That is the right amount of structure until external plugins exist.

## Risks

- There are two related catalogs: `src/cocli/tools/registry.rs` and `agent_skill_rows()` in dispatch. They serve different surfaces, but names can drift.
- JSON schemas are permissive (`additionalProperties: true`). This is fine for planning, not enough for remote tool execution.
- MCP transport is documented as gated/placeholder. Do not market it as a working server until a stdio smoke test exists.

## Test Plan

- `cargo test --all-targets`
- `target/debug/colab ai tools list`
- `target/debug/colab --json ai tools list`
- `target/debug/colab ai mcp` should fail with the experiment-gated message when disabled.
- Enable only in temp config before release and verify `settings skills mcp --json` lists tools without starting transport.

## YAGNI

- Skip external plugin abstractions until one real plugin must execute through this registry.
- Skip strict per-tool schemas while commands are only planned and inspected.
- Skip autonomous AI execution. Keep plans explicit, inspectable, and confirmation-gated.
