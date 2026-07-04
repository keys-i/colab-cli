# Kernel

Kernel controls live under `session` and `status`.

```sh
colab session kernel list
colab session kernel current
colab session kernel select
colab session kernel select python3
colab session kernel specs
colab session kernel start --spec julia-1.10
colab session kernel interrupt
colab session kernel restart --yes
colab session kernel shutdown --yes
colab session kernel refresh
colab status kernel --refresh
```

## Picking a Kernel

`session kernel select` stores the active kernel for the selected session.
Without an argument it opens a TTY picker. Outside a TTY, pass the kernel id or
name.

The picker shows selected marker, name, language, version, state, and a
shortened kernel id.

## Restarting and Interrupting

`session kernel interrupt` sends a Jupyter kernel interrupt. If the kernel is
busy, pass `--yes` or confirm interactively.

`session kernel restart` always needs `--yes` outside an interactive prompt.
Restarting loses variables and in-kernel state.

`session kernel shutdown` also needs confirmation because it may break the
current session.

## Language Detection

cocli detects language from Jupyter `kernel_info_reply` first:

1. `language_info.name`
2. kernelspec language
3. kernelspec or kernel name fallback
4. unknown

Detected language and version are cached in the local session record. Run
`colab session kernel refresh` to refresh kernels, kernelspecs, and
language info.

## Generic Package Tools

Use `run pkg` when you want the active kernel to decide the package tool:

```sh
colab run pkg add numpy pandas
colab run pkg list
colab run pkg update
colab run pkg restore requirements.txt
colab run pkg check
```

Python routes to pip. Julia routes to `Pkg`. R routes to base package helpers
and `renv` where requested.

If language is unknown:

```text
package tooling is not available for this kernel
fix: use `colab run code --code "..."`
```

## Python Tooling

When the cached active kernel is Python, `run --help` shows `run pip`:

```sh
colab run pip install torch
colab run pip freeze
colab run pip list
```

`run pip` is blocked when cached metadata says the active kernel is Julia or R.

## Julia Tooling

When the cached active kernel is Julia, use generic package commands:

```sh
colab run pkg add CSV DataFrames
colab run pkg status
colab run pkg restore
colab run pkg update
```

## R Tooling

When the cached active kernel is R, use generic package commands:

```sh
colab run pkg add dplyr
colab run pkg list
colab run pkg restore
colab run pkg status
```

Language-specific parser paths may exist for scripts, but public help should guide users through `run pkg`.

## Cache

Normal help does not call the network. It uses cached kernel metadata. If no
cache exists, help shows generic package tooling and this hint:

```text
kernel tools adapt after `colab session kernel refresh`
```
