# Logs

Session logs live under `session`.

```sh
colab-cli session logs --session trainer --tail 50
colab-cli session logs --session trainer --format text
colab-cli session logs --session trainer --format md --out logs.md
colab-cli session logs --session trainer --format jsonl --out logs.jsonl
colab-cli session logs --session trainer --format ipynb --out logs.ipynb
```

The hidden `log` alias maps to `session logs` for one migration cycle.

cocli does not fake logs. If a session has no persisted cocli execution log stream, it says that clearly. Remote runtime stdout/stderr still streams during `run` commands.
