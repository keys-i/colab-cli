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
