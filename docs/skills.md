# Skills

Skills are optional agent/tool surfaces exposed through `ai tools` and `settings skills`. Core product work stays in normal commands.

```sh
colab-cli ai tools list
colab-cli ai tools inspect slurp.plan
colab-cli settings skills list
colab-cli settings skills inspect slurp.plan
colab-cli settings skills run slurp.plan --json-input '{}'
```

Human output is a small catalog:

```text
Skill              Scope      Risk     Session   Network   Summary
slurp.plan         workflow   low      no        no        Explain a slurp.toml plan
slurp.explain      workflow   low      no        no        Render a clean Slurp plan explanation
fleet.plan         fleet      med      no        no        Plan approved runtime work
continue.resume    state      med      no        yes       Resume from checkpoint metadata
fs.diff            files      low      yes       yes       Compare local and remote trees
mcp.tools          agent      low      no        no        List MCP-compatible tool metadata
agent.audit        agent      low      no        no        Check a plan before running it
```

JSON output keeps stable field names:

```json
[
  {
    "name": "slurp.plan",
    "scope": "workflow",
    "category": "workflow",
    "risk": "low",
    "needs_session": false,
    "network": false,
    "summary": "Explain a slurp.toml plan"
  }
]
```

Running a skill returns a plan. It does not secretly execute network or destructive work.
