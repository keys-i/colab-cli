# google-colab-cli Map

Reference: `googlecolab/google-colab-cli`. cocli uses it as a feature reference only; no code is copied or vendored.

| google-colab-cli feature | cocli command | Status | Note |
|---|---|---|---|
| `new` / session creation | `colab-cli session new` | implemented | Runtime assignment with retryable Colab errors rendered cleanly. |
| `sessions` | `colab-cli session list` | implemented | Local store is reconciled when refreshed. |
| `status` | `colab-cli status`, `status session`, `status runtime` | implemented | Default status is local and cheap. |
| `restart-kernel` | `colab-cli session kernel restart --yes` | surface added | Command is confirmation-gated; backend implementation is deferred unless a safe Jupyter API path is available. |
| `stop` | `colab-cli session stop` | implemented | Removes local session and asks Colab to unassign. |
| `url` | `colab-cli session url --open` | implemented | Opens only when requested. |
| `run` | `colab-cli run script` | implemented | Runs through the selected runtime. |
| `exec` | `colab-cli run py`, `run script`, `run notebook` | implemented | Old `exec` alias is hidden. |
| `repl` | `colab-cli run repl` | implemented | Requires a selected session. |
| `console` | `colab-cli run shell` | implemented | Remote shell surface. |
| `ls` / `upload` / `download` / `rm` / `edit` | `colab-cli fs ...` | implemented | File operations stay under `fs`. |
| `auth` | `colab-cli auth login --method oauth2`, `auth login --method adc`, `auth status` | partial | OAuth2 is the normal login path; ADC detection is local and redacted. |
| `drivemount` | `colab-cli fs drive mount` | implemented | Staged preflight, timeout, retry, and friendly errors. |
| `install` | `colab-cli run pip install` | implemented | Old `run install` is a hidden migration alias. |
| `log` | `colab-cli session logs` | surface added | Reports persisted cocli logs when available; does not invent missing remote logs. |
| `pay` | `colab-cli settings billing open` | implemented as helper | Opens Colab billing page; local status may be unavailable. |
| `version` | `colab-cli status version`, `colab-cli --version`, `colab-cli settings about` | implemented | Includes version, build profile, features, and config path. |
| `update` | `colab-cli settings update check`, `settings update install --yes` | surface added | Check is local; install refuses blind self-modification. |

Compatibility aliases stay hidden from normal help. New docs use only the cocli command spaces.

## Interactive execution mapping

| google command | cocli command | Transport | Note |
|---|---|---|---|
| `colab repl` | `colab-cli run repl` | Jupyter kernel messaging | Local line editor sends code as kernel `execute_request`; piped stdin executes once. |
| `colab console` | `colab-cli run shell` | Colab `/colab/tty` PTY websocket | cocli does not assume `/api/terminals` exists for shell. Piped stdin sends `exit\n` on EOF. |
| `colab exec` | `colab-cli run py`, `run script`, `run notebook` | Runtime execution path | Core execution stays under `run`; old `exec` is a hidden alias. |
| `colab drivemount` | `colab-cli fs drive mount` | Kernel execution | Runs `google.colab.drive.mount()` in the attached Colab kernel and allows long browser approval time. |
| `colab log` | `colab-cli session logs` | Local cocli log/export surface | Does not invent unavailable remote logs. |
| `colab restart-kernel` | `colab-cli session kernel restart --yes` | Jupyter kernel API | Confirmation-gated because it interrupts work. |

Related transports:

| Area | Transport |
|---|---|
| Files | Jupyter Contents API where supported. |
| Kernel status/interrupt/restart | Jupyter kernel/session APIs where supported. |
| Drive approval | Kernel execution plus browser approval guidance; proprietary Colab credential propagation is not copied or vendored. |
| Auth | CLI auth stays under `auth`; VM-side browser credentials are only requested by explicit auth/Drive workflows. |
