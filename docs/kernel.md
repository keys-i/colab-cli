# Kernel

Kernel controls live under `session` and `status`.

```sh
colab-cli session kernel list
colab-cli session kernel current
colab-cli session kernel select
colab-cli session kernel select python3
colab-cli session kernel specs
colab-cli session kernel start --spec julia-1.10
colab-cli session kernel interrupt
colab-cli session kernel restart --yes
colab-cli session kernel shutdown --yes
colab-cli session kernel refresh
colab-cli status kernel --refresh
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
`colab-cli session kernel refresh` to refresh kernels, kernelspecs, and
language info.

## Generic Package Tools

Use `run pkg` when you want the active kernel to decide the package tool:

```sh
colab-cli run pkg add numpy pandas
colab-cli run pkg list
colab-cli run pkg update
colab-cli run pkg restore requirements.txt
colab-cli run pkg check
```

Python routes to pip. Julia routes to `Pkg`. R routes to base package helpers
and `renv` where requested.

If language is unknown:

```text
package tooling is not available for this kernel
fix: use `colab-cli run code --code "..."`
```

## Python Tooling

When the cached active kernel is Python, `run --help` shows `run pip`:

```sh
colab-cli run pip install torch
colab-cli run pip freeze
colab-cli run pip list
```

`run pip` is blocked when cached metadata says the active kernel is Julia or R.

## Julia Tooling

When the cached active kernel is Julia, `run --help` shows Julia tools:

```sh
colab-cli run julia pkg add CSV DataFrames
colab-cli run julia pkg status
colab-cli run julia pkg instantiate
colab-cli run julia pkg precompile
```

## R Tooling

When the cached active kernel is R, `run --help` shows R tools:

```sh
colab-cli run r pkg install dplyr
colab-cli run r pkg list
colab-cli run r renv restore
colab-cli run r session-info
```

## Cache

Normal help does not call the network. It uses cached kernel metadata. If no
cache exists, help shows generic package tooling and this hint:

```text
kernel tools adapt after `colab-cli session kernel refresh`
```
