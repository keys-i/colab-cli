# Skills

Skills are built-in command plans exposed through settings. They are for discovery and agent-friendly JSON, not external plugin loading.

```sh
colab-cli settings skills list
colab-cli settings skills inspect slurp.plan
colab-cli settings skills run slurp.plan --json-input '{}'
```

Human output is a small catalog:

```text
Skill                Risk   Scope      Needs session Summary
session.new          med    session    no            Start a runtime
run.python           med    run        yes           Run Python code
fs.sync              med    fs         yes           Plan file sync changes
slurp.plan           low    slurp      no            Explain a Slurp plan
fleet.plan           med    fleet      no            Plan approved runtimes
agent.audit          low    agent      no            Audit an agent plan
```

JSON output keeps stable field names:

```json
[
  {
    "name": "session.new",
    "scope": "session",
    "category": "session",
    "risk": "medium",
    "needs_session": false,
    "network": true,
    "dry_run": true,
    "summary": "Create a Colab session"
  }
]
```

Running a skill returns a plan. It does not secretly execute network or destructive work.
