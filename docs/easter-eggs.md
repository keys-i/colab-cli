# Easter Eggs

Fun output is tiny and opt-in or explicit.

Maintainer helper:

```sh
colab-cli settings set ui.fun true
```

`colab-cli release name v0.4.2` still parses as a hidden maintainer helper, but it is not part of the normal help surface.

Rules:

- never in `--json`
- never in `--quiet`
- never for auth failures, security warnings, or compliance refusals
- no security-sensitive randomness
- no giant banners

`ui.fun` is stored for future success-line plumbing.
