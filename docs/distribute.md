# Distribute

`distribute` is the experimental workflow area. It is off by default.

```sh
colab distribute plan
```

prints:

```text
experimental feature disabled: distribute
enable: colab settings experiments
```

Enable it explicitly:

```sh
colab settings experiments set distribute true
```

Names:

- `recipe`: tiny TOML workflow config, formerly the Slurp surface
- `pool`: approved runtime pool planning, formerly the Fleet surface
- `shard`: split work into safe chunks

Commands:

```sh
colab distribute plan
colab distribute status
colab distribute explain
colab distribute run --dry-run
colab distribute run --confirm
colab distribute resume
colab distribute clean

colab distribute recipe init
colab distribute recipe check
colab distribute recipe explain
colab distribute recipe run --dry-run
colab distribute recipe run --confirm

colab distribute pool plan
colab distribute pool status
colab distribute pool cost
colab distribute pool logs

colab distribute shard plan
colab distribute shard run --dry-run
colab distribute shard resume
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
