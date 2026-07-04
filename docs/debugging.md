# Debugging

Verbose mode is for debugging cocli, not normal use.

```sh
colab-cli -v status
colab-cli -vv fs drive mount
colab-cli -vvv fs drive mount
colab-cli -v run repl
colab-cli -v run shell
```

Levels:

- `-v` / `--verbose`: `debug1`, command, config, selected session, stages, retries.
- `-vv`: `debug2`, request method/path, timeouts, response status, short body summary.
- `-vvv`: `debug3`, sanitized full URLs, sanitized headers/bodies, internal error details.

Debug lines go to stderr and always start with `debug1:`, `debug2:`, or `debug3:`. Normal output stays on stdout. With `--json -v`, stdout remains valid JSON and debug lines stay on stderr. `--quiet` suppresses debug output.

Secrets are redacted before printing:

- bearer/access/refresh tokens
- cookies
- Colab proxy tokens
- OAuth secrets
- token-like query values, including `authuser`, `access_token`, `api_key`, and `key`
- the local home directory is shortened to `~`

Drive mount example:

```text
debug1: command fs.drive.mount
debug1: config loaded path=~/Library/Application Support/colab-cli/config.toml
debug1: session store loaded sessions=1 selected="<auto>"
debug1: selected session name="Colab CPU"
debug1: drive.mount stage=load_session ok name="Colab CPU"
debug1: drive.mount stage=validate_endpoint ok endpoint=m-s-...
debug1: drive.mount stage=check_jupyter_sessions attempt=1/3
debug2: http request method=GET path=/api/sessions timeout=10s
debug1: http timeout method=GET path=/api/sessions elapsed=10.003s retryable=yes
debug1: retry scheduled attempt=2/3 backoff=237ms
debug1: drive.mount failed kind=runtime_endpoint_timeout stage=check_jupyter_sessions retryable=yes
```

Attach `-vv` output to bug reports when possible. Use `-vvv` only when request details are needed; it is still redacted, but it is noisier.

Interactive transport examples:

```text
debug1: command run.shell
debug1: run.shell transport=colab_tty
debug1: run.shell websocket connecting path=/colab/tty
```

```text
debug1: command run.repl
debug1: run.repl stage=check_jupyter_sessions attempt=1/1
```

The REPL uses the Jupyter kernel websocket. Shell uses Colab's PTY websocket
where supported. Neither prints proxy tokens or full signed URLs in normal
human errors.
