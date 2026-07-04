# Distribute

`distribute` is the experimental workflow area. It is off by default.

```sh
colab-cli distribute plan
```

prints:

```text
experimental feature disabled: distribute
enable: colab-cli settings experiments
```

Enable it explicitly:

```sh
colab-cli settings experiments set distribute true
```

Names:

- `recipe`: tiny TOML workflow config, formerly the Slurp surface
- `pool`: approved runtime pool planning, formerly the Fleet surface
- `shard`: split work into safe chunks

Commands:

```sh
colab-cli distribute plan
colab-cli distribute status
colab-cli distribute explain
colab-cli distribute run --dry-run
colab-cli distribute run --confirm
colab-cli distribute resume
colab-cli distribute clean

colab-cli distribute recipe init
colab-cli distribute recipe check
colab-cli distribute recipe explain
colab-cli distribute recipe run --dry-run
colab-cli distribute recipe run --confirm

colab-cli distribute pool plan
colab-cli distribute pool status
colab-cli distribute pool cost
colab-cli distribute pool logs

colab-cli distribute shard plan
colab-cli distribute shard run --dry-run
colab-cli distribute shard resume
```

`cocli.recipe.toml` is preferred. Existing `slurp.toml` files still work as a fallback.

Compliance rules:

- no quota bypassing
- no account rotation when limited
- no free managed runtime pool claims
- multi-login is locked unless distribute is enabled
- real multi-runtime execution must use paid, enterprise, marketplace, or local profiles
- dry-run first
- JSON plans must stay stable

Hidden migration aliases:

```text
slurp -> distribute recipe
fleet -> distribute pool
```

Docs should use only `distribute`, `recipe`, `pool`, and `shard`.
