# google-colab-cli Map

Reference: `googlecolab/google-colab-cli`. cocli uses it as a feature reference only; no code is copied or vendored.

| google-colab-cli feature | cocli command | Status | Note |
|---|---|---|---|
| `new` / session creation | `colab session new` | implemented | Runtime assignment with retryable Colab errors rendered cleanly. |
| `sessions` | `colab session list` | implemented | Local store is reconciled when refreshed. |
| `status` | `colab status`, `status session`, `status runtime` | implemented | Default status is local and cheap. |
| `restart-kernel` | `colab session kernel restart --yes` | implemented | Uses Jupyter kernel API and is confirmation-gated because it loses in-kernel state. |
| `stop` | `colab session stop` | implemented | Removes local session and asks Colab to unassign. |
| `url` | `colab session url --open` | implemented | Opens only when requested. |
| `run` | `colab run script` | implemented | Runs through the selected runtime. |
| `exec` | `colab run py`, `run script`, `run notebook` | implemented | Old `exec` alias is hidden. |
| `repl` | `colab run repl` | implemented | Requires a selected session. |
| `console` | `colab run shell` | implemented | Remote shell surface. |
| `ls` / `upload` / `download` / `rm` / `edit` | `colab fs ...` | implemented | File operations stay under `fs`. |
| `auth` | `colab auth login --method oauth2`, `auth login --method adc`, `auth status` | partial | OAuth2 is the normal login path; ADC detection is local and redacted. |
| `drivemount` | `colab fs drive mount` | implemented | Staged preflight, timeout, retry, and friendly errors. |
| `install` | `colab run pip install` | implemented | Old `run install` is a hidden migration alias. |
| `log` | `colab log` | surface added | Reports persisted cocli logs when available; does not invent missing remote logs. |
| `pay` | `colab pay` | implemented as helper | Opens Colab billing page; local status may be unavailable. |
| `version` | `colab version`, `colab --version`, `colab status version` | implemented | Shows version; status/about can include build/config detail. |
| `update` | `colab update`, `colab update --install --yes` | surface added | Check is local; install refuses blind self-modification. |

Compatibility aliases stay hidden from normal help. New docs use only the cocli command spaces.

## Interactive execution mapping

| google command | cocli command | Transport | Note |
|---|---|---|---|
| `colab repl` | `colab run repl` | Jupyter kernel messaging | Local line editor sends code as kernel `execute_request`; piped stdin executes once. |
| `colab console` | `colab run shell` | Colab `/colab/tty` PTY websocket | cocli does not assume `/api/terminals` exists for shell. Piped stdin sends `exit\n` on EOF. |
| `colab exec` | `colab run py`, `run script`, `run notebook` | Runtime execution path | Core execution stays under `run`; old `exec` is a hidden alias. |
| `colab drivemount` | `colab fs drive mount` | Kernel execution | Runs `google.colab.drive.mount()` in the attached Colab kernel and allows long browser approval time. |
| `colab log` | `colab log` | Local cocli log/export surface | Does not invent unavailable remote logs. |
| `colab restart-kernel` | `colab session kernel restart --yes` | Jupyter kernel API | Confirmation-gated because it interrupts work. |
| kernel picker | `colab session kernel list/select/specs/current` | Jupyter kernel/session/kernelspec APIs | cocli caches detected language per local session. |

Related transports:

| Area | Transport |
|---|---|
| Files | Jupyter Contents API where supported. |
| Kernel status/interrupt/restart | Jupyter kernel/session APIs where supported. |
| Drive approval | Kernel execution plus browser approval guidance; proprietary Colab credential propagation is not copied or vendored. |
| Auth | CLI auth stays under `auth`; VM-side browser credentials are only requested by explicit auth/Drive workflows. |
