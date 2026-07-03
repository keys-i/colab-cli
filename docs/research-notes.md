# Research Notes

Primary sources:

- google-colab-cli README: https://github.com/googlecolab/google-colab-cli
- colabtools README: https://github.com/googlecolab/colabtools
- colab-mcp README: https://github.com/googlecolab/colab-mcp
- backend-info README: https://github.com/googlecolab/backend-info

`google-colab-cli` uses a flat `colab` command style: `new`, `sessions`, `status`, `stop`, `exec`, `repl`, `console`, `ls`, `upload`, `download`, `drivemount`, `install`, `log`, `pay`, `version`, `update`. It also forwards script arguments and supports local script execution. The Rust rewrite keeps cheap aliases and documents the new grouped command space.

`colab-mcp` is an MCP bridge from a local agent to a Colab browser session. The useful pattern is discoverable tools plus local-client requirements. This rewrite exposes tool specs and keeps execution behind CLI confirmation.

`colabtools` publishes Python libraries available inside Colab. Its README states the code is not intended for private reuse. This workspace learns concepts such as Drive mounting and runtime helpers but does not vendor implementation code.

`backend-info` publishes `apt-list.txt` and `pip-freeze.txt` snapshots. Its README says those files can lag production runtimes by one or two days. The CLI links to them as metadata, not as an exact live runtime promise.

Why checkpoint/replay: unrelated Colab runtimes do not share Python process memory. Continuation saves metadata, artifacts, and pending steps, then replays work on the same or a compatible new runtime.

Why no giant TUI: the requested workflows are command and automation heavy. Tables, JSON, and short status lines are enough.

Why terminal bell is opt-in: cross-platform terminal vibration is not real. The safe portable signal is `\x07`, disabled by default and never emitted in CI or quiet mode.

Removed or deferred:

- no new UI framework
- no async trait object tool registry
- no external watcher dependency for `fs sync --watch`
- no automatic pickle load path
