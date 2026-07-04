# Run

Run commands execute code or prepare a runtime.

```sh
colab-cli run py --session trainer --code "print(1)"
colab-cli run script train.py --session trainer -- --epochs 3
colab-cli run notebook report.ipynb --session trainer --out report.out.ipynb
colab-cli run repl --session trainer
colab-cli run shell --session trainer
echo "print('hello')" | colab-cli run repl --session trainer
echo "echo HELLO" | colab-cli run shell --session trainer
```

`run repl` uses Jupyter kernel execution. Input is read locally, then code is
sent as an `execute_request`, so arrow keys are handled by the local terminal
instead of leaking escape bytes into a remote `python` process. Piped stdin is
read once and executed once. Ctrl-C asks the remote kernel to interrupt; Ctrl-D
exits the local REPL.

`run shell` uses the Colab PTY websocket at `/colab/tty` where supported. It
does not assume Jupyter `/api/terminals` exists. In TTY mode cocli enters raw
mode only after the websocket connects and restores the terminal on exit. In
piped mode cocli forwards stdin, sends `exit\n`, waits briefly for output, and
closes the websocket so scripts do not hang forever.

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
