# Continuation

Continuation is checkpoint and replay.

What is saved:

- manifest version and timestamp
- session id and name
- runtime class and accelerator
- artifact paths
- executed and pending command steps
- mount metadata slots
- environment restore plan slots
- file manifest slots
- git commit and dirty-tree flag
- warnings

What is not saved:

- Python heap memory
- open sockets
- CUDA context state
- background process memory
- notebook variables unless the user explicitly wrote them to files

Resume order:

1. read the manifest
2. reconnect to the named session when possible
3. create a compatible new runtime only with `--new-runtime`
4. restore known files, env, and mounts when entries exist
5. replay pending steps by default
6. replay all steps only with `--replay-all`
7. write a resume report

Pickle checkpoints are user artifacts. Loading pickle is unsafe with untrusted files; the CLI must warn before adding any automatic pickle load path.
