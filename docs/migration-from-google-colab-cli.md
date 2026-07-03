# Migration from google-colab-cli

Old command examples:

```sh
colab new -s trainer --gpu A100
colab exec -s trainer -f train.py
colab upload -s trainer ./data.csv /content/data.csv
colab download -s trainer /content/out ./out
colab drivemount -s trainer
```

New forms:

```sh
colab-cli session new --name trainer --gpu A100
colab-cli exec run train.py --session trainer
colab-cli fs push ./data.csv /content/data.csv --session trainer
colab-cli fs pull /content/out ./out --session trainer
colab-cli mount drive --session trainer
```

Cheap compatibility aliases are kept for `new`, `sessions`, `status`, `stop`, `upload`, and `download`. They print migration hints.

Not carried over yet:

- transparent local file execution for `exec -f`
- edit-in-place over `$EDITOR`
- full session history export
- package install through `uv`

Those need live Colab testing before claiming parity.
