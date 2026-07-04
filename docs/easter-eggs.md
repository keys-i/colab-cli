# Easter Eggs

Fun output is tiny and opt-in or explicit.

Preference:

```sh
colab-cli settings set ui.fun true
```

Rules:

- never in `--json`
- never in `--quiet`
- never for auth failures, security warnings, or compliance refusals
- no security-sensitive randomness
- no giant banners

`ui.fun` is stored for future success-line plumbing.
