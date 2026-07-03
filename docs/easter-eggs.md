# Easter Eggs

Fun output is tiny and opt-in or explicit.

Commands:

```sh
colab-cli doctor --vibe
colab-cli doctor ferret
colab-cli release name v0.4.2
colab-cli config set ui.fun true
```

Rules:

- never in `--json`
- never in `--quiet`
- never for auth failures, security warnings, or compliance refusals
- no security-sensitive randomness
- no giant banners

`ui.fun` is stored for future success-line plumbing. The current release only uses explicit fun commands.
