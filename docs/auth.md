# Auth

Auth lives under `auth`.

```sh
colab auth login --method oauth2
colab auth login --method adc
colab auth status
colab auth list
colab auth logout
colab auth export-redacted
```

OAuth2 is the normal browser login path. Tokens must not be printed in logs, JSON, bug reports, or config.

ADC support is local detection:

```sh
colab auth login --method adc
```

If ADC credentials are missing, cocli prints:

```text
ADC credentials missing
fix: gcloud auth application-default login
```

Multi-login profile mutation is experimental and locked behind `distribute` plus `multi-login`. It is for legitimate profile selection, not account rotation to avoid limits.
