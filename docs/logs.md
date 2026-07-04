# Logs

Session logs live under `session`.

```sh
colab session logs --session trainer --tail 50
colab session logs --session trainer --format text
colab session logs --session trainer --format md --out logs.md
colab session logs --session trainer --format jsonl --out logs.jsonl
colab session logs --session trainer --format ipynb --out logs.ipynb
```

The hidden `log` alias maps to `session logs` for one migration cycle.

colab does not fake logs. If a session has no persisted colab execution log stream, it says that clearly. Remote runtime stdout/stderr still streams during `run` commands.
