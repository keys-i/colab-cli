# Plan

## What This Project Is

cocli is a Rust-first Colab CLI.

It should make normal Colab work less annoying:

- start or attach a session
- run code
- move files
- inspect runtime info
- manage Drive
- expose optional agent/code tools clearly
- checkpoint work only when the user enables that experiment
- plan distribute work only when the user enables that experiment

## What Done Looks Like

- [x] default command space follows `colab-cli <space> <command> <flags>`
- [x] default help shows `session`, `run`, `fs`, `status`, `ai`, `auth`, `settings`, and `completions`
- [x] no-command fallback prints help
- [x] `continue` is experimental/off by default
- [x] `distribute` replaces public `slurp` and `fleet`
- [x] `run pip` owns package commands
- [x] `session kernel` owns kernel list/select/specs/current/start/interrupt/restart/shutdown/refresh
- [x] `run pkg` routes package commands through the cached active kernel language
- [x] REPL prompt adapts to detected Python, Julia, R, or unknown kernels
- [x] status is human by default and JSON only with `--json`
- [x] JSON output has no ANSI in covered tests
- [x] AST observer is local/read-only and gated
- [x] `run repl` uses local input with remote kernel execution
- [x] `run shell` uses Colab `/colab/tty` where supported instead of assuming Jupyter terminals
- [x] settings interactive editing supports back/save/multiple edits
- [x] release helpers live under private `settings dev release`
- [ ] run live Drive/session smoke with a real Colab kernel
- [ ] replace the simple AST parser with Tree-sitter if exact nodes become necessary
- [ ] implement MCP stdio only when protocol tests exist
- [ ] run competitor benchmarks before making performance claims

## Current Priorities

1. Keep the command tree boring and stable.
2. Keep experiments off by default.
3. Keep distribute compliant and dry-run first.
4. Keep continuation honest as checkpoint/replay.
5. Benchmark before bragging.

## Non-Goals

- free-tier cluster behavior
- account rotation to bypass limits
- live Python memory migration
- plugin marketplace
- giant TUI
- hidden agent execution
- official Google replacement claim

## Risks

- Colab APIs and browser/runtime behaviour can change.
- Network benchmarks are noisy.
- Continuation can be oversold if wording gets sloppy.
- Multi-account support can accidentally look like quota bypassing.
- File sync can become complicated fast.

## Next Implementation Pass

- run the Drive live smoke once with browser approval
- run live REPL and shell smoke from a real terminal
- finish richer `fs sync` remote/cache comparison
- add exact AST parsing if the current outline is too rough
- implement MCP stdio only with protocol tests
- run first real competitor benchmark pass
- update claims ledger with measured numbers only
