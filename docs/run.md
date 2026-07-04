# Run

Run commands execute code or prepare a runtime.

```sh
colab-cli run py --session trainer --code "print(1)"
colab-cli run script train.py --session trainer -- --epochs 3
colab-cli run notebook report.ipynb --session trainer --out report.out.ipynb
colab-cli run repl --session trainer
colab-cli run shell --session trainer
```

Package commands live under `run pip`:

```sh
colab-cli run pip install torch transformers --session trainer
colab-cli run pip install -r requirements.txt --session trainer
colab-cli run pip freeze --session trainer
colab-cli run pip restore requirements.txt --session trainer
colab-cli run pip check --session trainer
colab-cli run pip list --session trainer
```

AST observer placement:

```sh
colab-cli settings experiments set ast-observer true
colab-cli run ast train.py
colab-cli run script train.py --ast --session trainer
colab-cli run notebook report.ipynb --ast --session trainer
```

`run ast` parses local files only. It does not start auth, create sessions, send source to an AI model, or execute code.
