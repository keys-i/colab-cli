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
Tool               Risk  Needs session  Network  Summary
recipe.plan        low   no             no       Explain a recipe plan
recipe.explain     low   no             no       Render a clean recipe explanation
distribute.plan    med   no             no       Plan approved runtime work
distribute.status  low   no             no       Show distribute planning status
fs.diff            low   yes            yes      Compare local and remote trees
ast.outline        low   no             no       Outline local Python code
mcp.tools          low   no             no       List MCP-compatible tool metadata
ai.audit           low   no             no       Check a plan before running it
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
    "summary": "Explain a recipe plan"
  }
]
```

Running a skill returns a plan. It does not secretly execute network or destructive work.
