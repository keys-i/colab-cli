# Use Cases

## Remote Python REPL

```sh
colab-cli session new
colab-cli run repl
```

Expected:

- arrow keys are handled locally
- Ctrl-C interrupts the remote kernel
- Ctrl-D exits the local REPL
- code runs through Jupyter kernel execution

## Remote Bash Shell

```sh
colab-cli run shell
```

Expected:

- connects to `/colab/tty` where supported
- raw mode starts only after the websocket connects
- terminal size changes are forwarded
- Ctrl-C goes to the remote PTY
- terminal state is restored on exit

## Pipe One Shell Command

```sh
echo 'echo HELLO' | colab-cli run shell
```

Expected:

- prints `HELLO`
- sends `exit\n` after stdin EOF
- exits without hanging forever

## Mount Drive

```sh
colab-cli fs drive mount
```

Expected:

- staged progress
- 600 second human-auth timeout
- browser approval guidance when needed
- no raw Python traceback for known Colab failures

## Expired Runtime Endpoint

```sh
colab-cli fs drive mount
```

Expected:

- friendly stale/unreachable endpoint error
- suggests `session list --refresh`, `session reconnect`, or `session new`
- raw request details only with `-vvv`

## Change Several Settings

```sh
colab-cli settings
```

Expected:

- edit multiple toggles before saving
- `s` saves once and stays open
- `b` or Esc navigates back
- no command preview or unsupported control hints

## Scripting

```sh
colab-cli status --json
```

Expected:

- valid JSON
- no ANSI
- no menus
- no animations
