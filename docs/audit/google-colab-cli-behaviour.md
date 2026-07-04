# google-colab-cli Behaviour Audit

Scope: local `google-colab-cli/` reference tree and installed `/Users/keys/miniconda3/bin/colab` observed on 2026-07-04. No code was changed.

Allowed live commands run: `colab -h`, `colab repl --help`, `colab console --help`, `colab auth --help`, `colab restart-kernel --help`, `colab update --help`, `colab pay --help`, `colab sessions`, `colab status`, `colab ls`, `colab log`, `colab drivemount`, `colab version`.

## Installed Behaviour Seen

- Installed version: `Version: 0.6.0`.
- `colab -h` works in the sandbox because it exits before logging; most subcommands initialize `~/.config/colab/colab.log` first, so sandboxed help failed with `PermissionError` until run outside the sandbox.
- Active live session observed: `[046ae3] m-s-kkb-use4a1-2uiyb7m7ubqzc | Hardware: CPU | Variant: DEFAULT`.
- `colab status` prints the same line with `Status: IDLE` and a last execution line.
- `colab ls` auto-selected the unique session, printed `[colab] Using unique session '046ae3'.`, then listed `.config/`, `drive/`, `sample_data/`.
- `colab log` with no session listed sessions with history: `046ae3`, `0ffd0d`.
- `colab drivemount` auto-selected the unique session, intercepted Drive auth, propagated credentials, and reported Drive already mounted at `/content/drive`.

## Public Surface

Top-level help is flat, Typer/Rich styled, and alphabetized. Public commands shown by `colab -h`:

`console`, `download`, `drivemount`, `edit`, `exec`, `help`, `install`, `log`, `ls`, `new`, `pay`, `readme`, `repl`, `restart-kernel`, `rm`, `run`, `sessions`, `skill`, `status`, `stop`, `update`, `upload`, `url`, `version`.

Global options:

- `--client-oauth-config/-c`
- `--config`
- `--logtostderr`
- `--auth [oauth2|adc]`
- completion options
- `--help/-h`

Hidden but reachable in source: `auth`, `whoami`, `keep-alive`, uppercase `README`, uppercase `SKILL`.

## Auth

Host auth has two providers:

- `oauth2`: public installed-app OAuth with scopes `openid`, user profile/email, `cloud-platform`, `colaboratory`, and `drive.file`.
- `adc`: Google Application Default Credentials with the same Colab-required scopes.

OAuth2 uses a remote copy-paste flow through `https://sdk.cloud.google.com/applicationdefaultauthcode.html`, not localhost and not OOB. Tokens are stored at `~/.config/colab/token.json`; session state is `~/.config/colab/sessions.json`; settings are `~/.config/colab/settings.json`.

Safe cocli difference: keeping PKCE localhost OAuth is fine, but document it as a deliberate UX difference. Do not claim ADC parity unless ADC credentials actually feed Colab API requests with the required scopes.

## Drive

`colab drivemount [-s SESSION] [PATH=/content/drive]` executes `from google.colab import drive; drive.mount(PATH)` in the remote kernel with a 600 second interactive timeout.

Important behaviour: google-colab-cli intercepts kernel `colab_request` messages with `authType=dfs_ephemeral`, calls `/tun/m/credentials-propagation/<endpoint>` dry-run and real propagation requests, may print a browser approval URL, then replies on the kernel stdin channel with the matching `colab_msg_id`.

Safe cocli difference: cocli may keep the browser-open fallback, but should not report Drive as fully CLI-native until it implements credentials propagation and confirms mount status remotely.

## REPL

`colab repl` options: `--session/-s`, `--output-image`.

TTY input starts a prompt-toolkit Python REPL with `>>>`, multiline via `Esc Enter` or `Ctrl-J`, `/quit`, `quit()`, and `exit()`. Piped stdin executes once through the same Jupyter runtime path, records history as `source=piped`, and exits cleanly on empty input.

Safe cocli difference: `run repl` is a better grouped name. Keep piped stdin semantics only if they are useful; otherwise direct users to `run code` or `run script`.

## Console

`colab console` options: `--session/-s`.

It connects to `/colab/tty?colab-runtime-proxy-token=...` over websocket, puts TTY stdin in raw mode, sends terminal size, streams raw ANSI data, and restores terminal settings in `finally`. For piped stdin, EOF sends `exit\n`, waits briefly, and closes the websocket to avoid hanging on tmux/bash.

Safe cocli difference: `run shell` is a clearer grouped name. Preserve raw terminal restoration and piped EOF handling.

## Log

`colab log` options in source: `--session/-s`, `--lines/-n`, `--type/-t`, `--output/-o`.

No session lists sessions with local history. With a session, it prints compact event lines for executions, file operations, automation, stdin/input replies, and keep-alive lifecycle. Export format is inferred from output suffix: `.ipynb`, `.md`, `.txt`, `.jsonl`.

Safe cocli difference: grouped `log list/show/export/tail` is fine. Do not invent server logs; clearly label absent persisted logs.

## Update

`colab update` checks `https://pypi.org/pypi/google-colab-cli/json`. `--install` self-upgrades with pip or uv where supported. Normal commands run a daily background update check unless disabled by `"enable_update_check": false` in settings.

Banner suppression is intentional for pipeable/display commands: `update`, `version`, `log`, `pay`, `help`, `url`, `whoami`, `readme`, `skill`.

Safe cocli difference: requiring explicit `--yes` for self-install is safer. Keep update output out of pipeable commands.

## Pay and Version

`colab pay` opens `https://colab.research.google.com/signup` and prints `[colab] Opening ...`.

`colab version` prints `Version: <version>` only.

Safe cocli difference: `pay --dry-run` and JSON output are useful additions. Keep a plain pipeable version command.

## Stop, URL, Restart Kernel

`colab stop [-s SESSION]` resolves a session, kills the keep-alive pid if present, attempts Jupyter shutdown, unassigns the Colab endpoint, removes local state, logs `session_terminated`, and prints `[colab] Session terminated.` Missing sessions are non-fatal.

`colab url [-s SESSION] [--host HOST] [--open]` prints a connect URL on its own line. The URL uses `/notebooks/empty.ipynb?dbu=%2Ftun%2Fm%2F<endpoint>#datalabBackendUrl=<host>/tun/m/<endpoint>`. Browser opening is opt-in so output stays pipeable.

`colab restart-kernel [-s SESSION]` sends a kernel restart through the Jupyter runtime and stops the runtime client afterward. It has no confirmation flag.

Safe cocli difference: `session stop`, `session url`, and `session kernel restart --yes` are safer grouped forms. Keep compatibility aliases only where cheap and documented as migration aids.

## UX Style

- Output is terse and mostly line-oriented.
- Status/session lines use `[name] endpoint | Hardware: CPU | Variant: DEFAULT | Status: IDLE`.
- Human messages usually start with `[colab]`.
- Raw values intended for piping, especially `url` and `version`, avoid noisy banners.
- Missing unique session paths use shared resolution: no sessions gives a create-one hint; one active session is auto-selected; multiple sessions require `-s`.
- Errors are often friendly but not always structured; Typer tracebacks can leak for early setup failures like log-file permissions.

Safe cocli difference: cocli's grouped command tree, JSON mode, confirmations, and richer error hints are improvements. Avoid copying Typer tracebacks or unsolicited update banners into machine-readable output.

## Safe Differences for cocli

- Keep cocli's primary surface grouped: `session`, `run`, `fs`, `status`, `auth`, `log`, `settings`, `ai`.
- Keep flat google-colab-cli names hidden or migration-only: `new`, `sessions`, `stop`, `upload`, `download`.
- Do not add more flat aliases unless live migration demand justifies them.
- Keep `session url` pipeable and browser opening opt-in.
- Keep `session kernel restart --yes`; google-colab-cli's no-confirm restart is not worth copying.
- Keep Drive mount honest: success requires a remote mount probe, not just starting the cell.
- Keep ADC labelled detection-only until it actually authenticates Colab API calls.
- Keep logs local and redacted; do not claim Colab server log access.
- Keep update install explicit and confirmed.
