# Output Style

Both tools use one output rule: human output is short; JSON is machine-only.

## Color Roles

| Role | Use |
|---|---|
| success | completed safe action |
| warning | recoverable risk |
| error | blocked action |
| info | neutral state, cyan |
| command | commands and active selections, cyan |
| path | paths, blue/lavender |
| muted | secondary detail, grey |
| accent | headings and IDs, violet/blue |

No raw ANSI in JSON. Respect `NO_COLOR`, `--no-color`, `--quiet`, and CI. `--color always` is the explicit override for piped human output.

The no-command launcher is interactive-only. In pipes or scripts, output stays plain and predictable.

## Formats

Error:

```text
error: what failed
fix: exact command to try
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

No giant banners. Fun lines never appear in JSON, quiet output, security failures, auth failures, or data-loss paths.

No Quick Actions, command previews, or key hints are printed unless the command actually implements that interaction.
