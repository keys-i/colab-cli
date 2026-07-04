# userdata.get in CLI-run Colab Code

## Summary

`google.colab.userdata.get()` can time out in CLI-run code because Colab web UI
Secrets rely on the frontend message path.

## Steps To Reproduce

```sh
colab run --timeout 30 script.py
```

`script.py`:

```py
from google.colab import userdata
token = userdata.get("HF_TOKEN")
```

The same code can work in the Colab web UI when the secret is configured in the
sidebar.

## Expected Behavior

CLI-run code should have a safe way to receive API keys explicitly supplied by
the user.

## Current Workarounds

- shell-exporting secrets into command strings can leak into history
- dotenv uploads are clunky
- hardcoding secrets is unsafe

## Proposed Solution

Add an experimental CLI secrets bridge:

- `colab run ... --env KEY`
- `colab run ... --secret KEY`
- `colab run ... --env-file PATH`
- `colab secret ...` for local secret command surface
- redaction in logs, verbose output, JSON, and support bundles where possible

## Environment From Report

- colab version: 0.5.9
- OS: WSL2 Ubuntu 24.04
- auth: oauth2
- subscription: Colab Pro+

## colab Design Decision

colab does not claim access to Colab web UI sidebar secrets. The bridge only
uses secrets explicitly supplied from the local environment, env files, or
future keyring-backed local storage.

For the MVP, Python execution receives a small local prelude that sets
`os.environ`, and the kernel websocket loop answers Colab `GetSecret` requests
with explicitly supplied local secrets.

## Implementation Status

- env and env-file parsing: implemented
- secret value redaction type: implemented
- run command env injection: implemented
- Python `userdata.get` prelude bridge: implemented
- kernel `GetSecret` request/reply bridge: implemented
- persistent keyring storage: deferred
- live Colab secret smoke: not run by default

## Tests

Unit and CLI tests cover parsing, redaction, gate behavior, dotenv parsing, and
userdata reply shape. Live testing requires:

```sh
COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 ./scripts/live-secrets-smoke.sh
```
