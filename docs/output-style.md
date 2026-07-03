# Output Style

Both tools use one output rule: human output is short; JSON is machine-only.

## Color Roles

| Role | Use |
|---|---|
| success | completed safe action |
| warning | recoverable risk |
| error | blocked action |
| info | neutral state |
| muted | paths and secondary detail |
| accent | command names and IDs |

No raw ANSI in JSON. Respect `NO_COLOR`, `--quiet`, and CI.

## Formats

Error:

```text
error: what failed
next: exact command to try
```

Dry-run:

```text
dry run: action summary
would change: N item(s)
```

Table:

```text
name       status   next
trainer    ready    run script
```

No giant banners. Animal/fun lines never appear in JSON, quiet output, security failures, auth failures, or data-loss paths.
