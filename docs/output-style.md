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

No raw ANSI in JSON. Respect `NO_COLOR`, `--no-color`, `--quiet`, and CI. Persistent colour mode is configured with `colab settings ui set color auto|always|never`.

Running `colab` with no args prints help. It does not open a launcher.

Verbose diagnostics use `-v`, `-vv`, and `-vvv`. Debug lines go to stderr, are prefixed with `debug1:`, `debug2:`, or `debug3:`, and are not mixed into JSON stdout.

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
