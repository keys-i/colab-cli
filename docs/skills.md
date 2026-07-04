# Skills

Skills are optional agent/tool surfaces exposed through `ai tools` and `settings skills`. Core product work stays in normal commands.

```sh
colab-cli ai tools list
colab-cli ai tools inspect recipe.plan
colab-cli settings skills list
colab-cli settings skills inspect recipe.plan
colab-cli settings skills run recipe.plan --json-input '{}'
```

Human output is a small catalog:

```text
Tool               Risk  Session  Network  State  Summary
recipe.plan        low   no       no       ready  Explain a recipe plan
recipe.explain     low   no       no       ready  Render a clean recipe explanation
distribute.plan    med   no       no       gated  Plan approved runtime work
distribute.status  low   no       no       gated  Show distribute planning status
fs.diff            low   yes      yes      ready  Compare local and remote trees
fs.changed         low   yes      yes      ready  Show local changes that sync would upload
ast.outline        low   no       no       off    Outline local Python code
mcp.tools          low   no       no       off    List MCP-compatible tool metadata
ai.audit           low   no       no       ready  Check a plan before running it
```

`continue.save` and `continue.resume` appear only when the Continue experiment is enabled.

JSON output keeps stable field names:

```json
[
  {
    "name": "recipe.plan",
    "scope": "workflow",
    "category": "workflow",
    "risk": "low",
    "needs_session": false,
    "network": false,
    "state": "ready",
    "summary": "Explain a recipe plan"
  }
]
```

Running a skill returns a plan. It does not secretly execute network or destructive work.
