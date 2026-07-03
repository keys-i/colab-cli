# Skills

The old top-level tool surface moved under settings:

```sh
colab-cli settings skills list
colab-cli settings skills inspect fs.push
colab-cli settings skills run fs.push --json '{"src":"./data.csv","dest":"/content/data.csv"}'
```

The registry is small and built in. It exists so humans and agents can inspect safe command plans without loading network clients.

## Names

- `session.new`
- `session.status`
- `run.python`
- `run.notebook`
- `run.install`
- `fs.list`
- `fs.push`
- `fs.pull`
- `drive.mount`
- `continue.save`
- `continue.resume`
- `runtime.info`
- `status.check`

## Human Output

```text
Skill              Risk     Needs session Network Dry-run Summary
session.new        medium   no            yes     yes     Start a Colab runtime
run.python         medium   yes           yes     yes     Run Python code
fs.push            medium   yes           yes     yes     Upload files
status.check       low      no            no      yes     Check local setup
```

## JSON Shape

```json
[
  {
    "name": "session.new",
    "category": "session",
    "risk": "medium",
    "needs_session": false,
    "network": true,
    "dry_run": true,
    "summary": "Start a runtime"
  }
]
```

External plugin loading is deferred. There is no tested external plugin contract yet.
