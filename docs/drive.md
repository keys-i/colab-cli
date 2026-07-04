# Drive

Drive lives under `fs` because it changes the runtime filesystem.

```sh
colab-cli fs drive mount --session trainer
colab-cli fs drive mount --session trainer --timeout 180 --preflight-timeout 10 --retries 2
colab-cli fs drive status --session trainer
colab-cli fs drive list --session trainer
colab-cli fs drive unmount --session trainer
colab-cli fs drive path --session trainer
```

## Mount Flow

`fs drive mount` is staged:

1. load selected session
2. validate endpoint URL
3. check endpoint reachability
4. check Jupyter sessions
5. find a kernel
6. check existing Drive mount
7. verify kernel context
8. request Drive mount
9. wait for browser approval if needed
10. verify `/content/drive`

If the endpoint is unreachable, cocli stops before kernel checks:

```text
Drive mount failed

Runtime endpoint is not reachable
stage: check_jupyter_sessions
retryable: yes

fix: colab-cli session list --refresh
     colab-cli session new --name work

Use --verbose to see the request details
```

Human mode strips raw reqwest/HTML/traceback walls. `--verbose` keeps trimmed request details. `--json` returns structured error fields and no ANSI.

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
Drive mount failed

Drive mount needs a Colab kernel session, not a plain Python process
stage: verify_kernel_context

fix: colab-cli session url --open
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

## Recovery Commands

```sh
colab-cli session refresh
colab-cli session repair --session trainer
colab-cli session reconnect --session trainer
colab-cli session last
```

These commands help detect stale local endpoint records. They do not create hidden runtimes or bypass Colab limits.
