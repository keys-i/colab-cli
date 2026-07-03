# Drive

Drive lives under `fs` because it changes the runtime filesystem.

```sh
colab-cli fs drive mount --session trainer
colab-cli fs drive status --session trainer
colab-cli fs drive list --session trainer
colab-cli fs drive unmount --session trainer
colab-cli fs drive path --session trainer
```

## Kernel Requirement

`google.colab.drive.mount()` only works inside a Colab/IPython kernel. It is not safe to run it through a plain remote `python -c` process.

cocli now runs Drive mount through the kernel execution path. Before mounting it checks that the attached session has a kernel:

```python
import IPython
ip = IPython.get_ipython()
print(hasattr(ip, "kernel"))
```

If that check fails, the command prints:

```text
Drive mount needs a Colab kernel session, not a plain Python process
next: colab-cli session url --open
```

Open the session URL once, approve Drive in the browser if Colab asks, then run the mount command again.

## Browser Approval

Drive auth can require browser approval. Use:

```sh
colab-cli fs drive mount --session trainer --open
```

The CLI will not wait forever. If approval is still pending at the timeout, it exits with a next action instead of dumping a Python traceback.

## Enterprise And Unsupported Runtimes

Some Colab Enterprise setups do not support `google.colab.drive.mount()`. cocli reports that as a normal Drive error:

```text
Drive mount is not supported for this runtime
next: colab-cli status check
```

Verbose mode can show the raw traceback when it is needed for debugging:

```sh
colab-cli --verbose fs drive mount --session trainer
```

JSON mode uses structured fields and no ANSI codes.
