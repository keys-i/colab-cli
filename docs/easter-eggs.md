# Easter Eggs

Fun output is tiny and opt-in or explicit.

Maintainer helper:

```sh
colab-cli settings set ui.fun true
```

`colab-cli settings owner release name` exists only when built with `--features owner-tools`.

Rules:

- never in `--json`
- never in `--quiet`
- never for auth failures, security warnings, or compliance refusals
- no security-sensitive randomness
- no giant banners

`ui.fun` is stored for future success-line plumbing.
