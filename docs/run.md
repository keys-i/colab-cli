# Run

Run commands execute code or prepare a runtime.

```sh
colab run py --session trainer --code "print(1)"
colab run script train.py --session trainer -- --epochs 3
colab run notebook report.ipynb --session trainer --out report.out.ipynb
colab run repl --session trainer
colab run shell --session trainer
echo "print('hello')" | colab run repl --session trainer
echo "echo HELLO" | colab run shell --session trainer
```

`run repl` uses Jupyter kernel execution. Input is read locally, then code is
sent as an `execute_request`, so arrow keys are handled by the local terminal
instead of leaking escape bytes into a remote `python` process. Piped stdin is
read once and executed once. Ctrl-C asks the remote kernel to interrupt; Ctrl-D
exits the local REPL.

`run shell` uses the Colab PTY websocket at `/colab/tty` where supported. It
does not assume Jupyter `/api/terminals` exists. In TTY mode colab enters raw
mode only after the websocket connects and restores the terminal on exit. In
piped mode colab forwards stdin, sends `exit\n`, waits briefly for output, and
closes the websocket so scripts do not hang forever.

Generic package commands follow the selected kernel:

```sh
colab run pkg add numpy pandas --session trainer
colab run pkg list --session trainer
colab run pkg update --session trainer
colab run pkg restore requirements.txt --session trainer
```

For Python kernels, advanced package commands live under `run pip`:

```sh
colab run pip install torch transformers --session trainer
colab run pip install -r requirements.txt --session trainer
colab run pip freeze --session trainer
colab run pip restore requirements.txt --session trainer
colab run pip check --session trainer
colab run pip list --session trainer
```

Secrets bridge is experimental and off by default:

```sh
colab settings experiments set secrets-bridge true
colab run script train.py --env HF_TOKEN --session trainer
colab run py --env HF_TOKEN --code "from google.colab import userdata; print(len(userdata.get('HF_TOKEN')))"
```

`--env KEY` reads local environment variable `KEY` and exposes it to the remote
run as `os.environ["KEY"]`. For Python runs, colab also bridges
`google.colab.userdata.get("KEY")` for keys supplied to that run.

`--env REMOTE:LOCAL` maps a local env var to a different remote name.
`--env-file PATH` reads a dotenv-style file. `--env KEY=VALUE` is accepted for
automation but can leak through shell history; prefer `--env KEY` or
`--secret KEY`.

For Julia and R kernels:

```sh
colab run pkg add CSV DataFrames --session trainer
colab run pkg status --session trainer
colab run pkg add dplyr --session trainer
colab run pkg restore --session trainer
```

If cached kernel metadata says the active kernel is Julia or R, `run pip` is
blocked with a short wrong-language error. Use `run pkg` for portable package
commands. Language-specific parser paths are compatibility/internal unless
dynamic help exposes them from cached kernel metadata.

AST observer placement:

```sh
colab settings experiments set ast-observer true
colab run ast train.py
colab run script train.py --ast --session trainer
colab run notebook report.ipynb --ast --session trainer
```

`run ast` parses local files only. It does not start auth, create sessions, send source to an AI model, or execute code.
