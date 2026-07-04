# Skills

Skills are optional agent/tool surfaces exposed through `ai tools` and `settings skills`. Core product work stays in normal commands.

```sh
colab ai tools list
colab ai tools inspect runtime.inspect
colab settings skills list
colab settings skills inspect runtime.inspect
colab settings skills run agent.audit --json-input '{}'
```

Human output is a small catalog:

```text
Tool               Risk  Session  Network  State  Summary
runtime.inspect    low   yes      yes      ready  Inspect runtime metadata
fs.diff            low   yes      yes      ready  Compare local and remote trees
fs.changed         low   yes      yes      ready  Show local changes that sync would upload
support.bug-report low   no       no       ready  Write a redacted diagnostic bundle
ai.audit           low   no       no       ready  Check a plan before running it
```

`recipe.*`, `distribute.*`, `continue.*`, `ast.*`, and `mcp.*` rows appear only when their experiments are enabled.

JSON output keeps stable field names:

```json
[
  {
    "name": "runtime.inspect",
    "scope": "runtime",
    "category": "runtime",
    "risk": "low",
    "needs_session": true,
    "network": true,
    "state": "ready",
    "summary": "Inspect runtime metadata"
  }
]
```

Running a skill returns a plan. It does not secretly execute network or destructive work.
