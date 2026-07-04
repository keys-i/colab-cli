# Security Review

Date: 2026-07-04

## Scope

Reviewed auth, Drive, secrets, verbose logging, support output, and session logs
for obvious user-visible leaks.

## Findings

| Area | Finding | Status |
|---|---|---|
| Secret values | `SecretValue` redacts `Debug` and `Display`; CLI tests cover exact token redaction. | pass |
| Secrets bridge | Hidden/off by default; run flags are parsed but gated before injection unless experiment is enabled. | pass |
| JSON output | CLI tests assert JSON output has no ANSI. | pass |
| Verbose output | Debug URL rendering uses `debug::sanitize_url`; raw reqwest URL display is avoided in normal errors. | pass with live caveat |
| Drive OAuth URL | User-facing Drive auth may need to show the real approval URL. Logs/support must redact it. | needs live smoke |
| OAuth tokens | Auth docs and storage tests cover token redaction and no token printing. | pass |
| Support bundles | Redacted support command exists; no current test here proves every new secret path is covered. | follow-up |
| Session logs | Logs are intended to redact secrets; live run/log smoke was not executed in this pass. | follow-up |
| Hidden agent alias | Old `agent plan` executed without new gates. | fixed |

## Rules Kept

- No plaintext secret values in config.
- No secret values in JSON, verbose output, AI tool output, or logs by design.
- No quota-bypass language or hidden multi-login behavior.
- No raw HTML/network/traceback walls in normal human errors.

## Required Before Stable Secrets/Drive Claims

Run live smoke with explicit test tokens:

```text
COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 ./scripts/live-secrets-smoke.sh
COLAB_CLI_LIVE=1 ./scripts/live-smoke.sh
```

Do not use real production tokens for smoke tests.
