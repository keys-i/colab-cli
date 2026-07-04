# Secrets Bridge

## Design

Secrets bridge is an experimental local bridge for CLI-run code. It passes
explicit local secrets into a Colab run, patches Python `userdata.get` where
needed, and answers Colab kernel `GetSecret` requests when they appear.

It does not read or sync Colab web UI sidebar secrets.

Sources, in order:

1. `--env KEY`, `--env REMOTE:LOCAL`, or explicit `--env KEY=VALUE`
2. `--secret KEY` or `--secret REMOTE=LOCAL`, using local env fallback
3. `--env-file PATH`

No secret values are written to `config.toml`.

## Motivation

Colab web UI Secrets are frontend-backed. In CLI mode there is no browser
frontend answering `GetSecret` requests, so `userdata.get("KEY")` can time out.
The bridge supplies local secrets the user explicitly provides to the CLI.

## Shebang Usage

Preferred:

```sh
export HF_TOKEN=hf_...
colab settings experiments set secrets-bridge true
colab run script train.py --env HF_TOKEN
```

Prompt or local store command surface:

```sh
colab secret set WANDB_API_KEY --prompt
colab run script train.py --secret WANDB_API_KEY
```

In the current experimental build, `secret set` does not fall back to plaintext
storage. Use `--env`, `--secret` with local env fallback, or `--env-file`.

Dotenv:

```sh
colab run script train.py --env-file .env
```

Dotenv files are convenient, but keyring or local environment variables are
safer.

Avoid:

```sh
colab run py --env HF_TOKEN=hf_...
```

That form can leak through shell history or process listings. It is accepted
for automation, warns in human mode, and is redacted from output.

## Behavior

When enabled, run commands can inject selected secrets:

```sh
colab run py --env HF_TOKEN --code "from google.colab import userdata; print(len(userdata.get('HF_TOKEN')))"
colab run script train.py --env HF_TOKEN
colab run repl --env HF_TOKEN
colab run shell --env HF_TOKEN
colab run pkg add transformers --env HF_TOKEN
```

Python code gets:

- `os.environ["KEY"]`
- `google.colab.userdata.get("KEY")` for keys supplied to this run

Shell and package commands get environment variables. Values are never printed
by cocli.

If the experiment is disabled:

```text
experimental feature disabled: secrets bridge
enable: colab settings experiments
```

If a key is missing:

```text
Missing secret: HF_TOKEN
fix: export HF_TOKEN=... or run colab secret set HF_TOKEN --prompt
```

## AGENTS.md Constraints

- Do not forward all local environment variables.
- Do not expose secret values in AI/MCP tool output.
- Use secret names only.
- Require explicit user approval before requesting a secret for an agent run.
- Do not store plaintext secrets in config.

## Testing Strategy (TDD)

Covered tests:

- `SecretValue` debug/display redaction
- key validation for empty and whitespace keys
- dotenv parsing for comments and quoted values
- `--env KEY`, `--env REMOTE:LOCAL`, and `--env KEY=VALUE` mapping
- missing env gives a short fix
- userdata reply shape for present and missing keys
- exact-value and common-token redaction
- command parsing for `colab secret` and run `--env`
- experiment gate before secret injection
- AI tool catalog includes secret wrappers only when enabled

Live smoke is optional:

```sh
COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 ./scripts/live-secrets-smoke.sh
```

Do not mark the bridge stable until a live Colab run verifies `userdata.get`
without printing the secret value.
