# Research

## Why I Looked At The Google Tools

cocli is not trying to act like Google's tools don't exist. The useful work is to read them, keep the parts that make sense, and build a sharper Rust CLI around the gaps this project cares about.

The main things I wanted from the references were command clarity, file sync, continuation, Slurp config, stable JSON, useful doctor commands, and safer account/fleet boundaries.

`google-colab-cli` already covers a lot of normal work: session/runtime commands, execution, file movement, installs, logs, Drive mounting, and agent-style workflows. `colab-mcp` pushed the agent/tool direction: make tools discoverable and make state explicit. `colabtools` and `backend-info` are useful for runtime metadata thinking, but they are not a code source for this project.

cocli should not copy private or internal Google implementation code. It also should not pretend to be official Google software.

## What I Copied Conceptually, Not Literally

- session lifecycle
- running code remotely
- file movement
- runtime info
- agent-friendly tool surfaces
- doctor/debug commands

No vendored Google implementation.

No official affiliation.

No quota-bypass features.

## What I Changed

The command shape is:

```sh
colab-cli <space> <command> <flags>
```

That is a little longer than `colab exec`, but it gives the command tree room to stay readable. `session`, `exec`, `fs`, `continue`, `slurp`, and `doctor` are separate because they answer different user questions.

I kept the Rust code as one internal `src/cocli/` module tree. The Rust Book's module guidance fits this better than a pile of small crates right now: group code by responsibility, keep details private, and extract later when a boundary is real. Cargo workspaces are useful, but they add release and versioning work. This project does not have a stable public API split yet.

Slurp is a small TOML orchestration file, not a workflow platform. It should explain what will happen before it runs.

Continuation is checkpoint/replay. It saves files, metadata, command state, artifacts, and pending steps. It does not move live Python memory between unrelated Colab runtimes.

File sync should avoid re-uploading unchanged work. That claim still needs real remote-manifest measurements before the README can say anything stronger.

Doctor commands should tell users what to run next. A stack trace is not a diagnosis.

## What I Refused To Build

- free-tier Colab cluster mode
- account rotation to bypass limits
- live kernel memory teleportation
- plugin marketplace
- giant TUI
- fake terminal vibration
- "agent runs everything for you" mode

Some of this sounded cool at first, but it creates more problems than it solves. The CLI should help with normal Colab work, not encourage policy problems or hide risky actions.

## Benchmark Thinking

The benchmark targets are `google-colab-cli`, manual upload/download flows, plain notebook execution, and local deterministic paths.

Measured numbers only. A 10x or 50x claim needs a table in `docs/claims-ledger.md`. If the competitor is not installed, or the real Colab API is not tested, the result stays "not proven".

Local benchmarks and real Colab/network benchmarks are separate. Local tests can measure startup, parsing, manifest diffing, and JSON cleanliness. Network tests are useful, but noisy.

## Usability Thinking

The questions are plain:

- Where is my session?
- What changed?
- Can I dry-run this?
- How do I resume?
- What broke, and what do I run next?

Nielsen's heuristics are useful here because they push toward visible status, fewer memory games, clear recovery text, and less screen junk. The CLI should help at 2am, not show off.

## Sources

[google-colab-cli]: https://github.com/googlecolab/google-colab-cli
[colabtools]: https://github.com/googlecolab/colabtools
[colab-mcp]: https://github.com/googlecolab/colab-mcp
[backend-info]: https://github.com/googlecolab/backend-info
[rust-modules]: https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
[cargo-workspaces]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[rust-api-docs]: https://rust-lang.github.io/api-guidelines/documentation.html
[nielsen-heuristics]: https://www.nngroup.com/articles/ten-usability-heuristics/
[claims-ledger]: claims-ledger.md
[benchmark-results]: benchmark-results.md
