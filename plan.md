# Plan

## What This Project Is

cocli is a Rust-first Colab CLI.

It should make normal Colab work less annoying:

- start or attach a session
- run code
- move files
- inspect runtime info
- checkpoint work
- resume safely
- explain what it is about to do

## What Done Looks Like

- [ ] command space follows `colab-cli <space> <command> <flags>`
- [ ] `session`, `run`, `fs`, `status`, `continue`, `slurp`, and `settings` cover the main path
- [ ] `fs sync --dry-run` is trustworthy
- [ ] continuation is checkpoint/replay and says so clearly
- [ ] Slurp config can be explained before it runs
- [ ] status commands give one next action
- [ ] JSON output has no ANSI
- [ ] fun output never appears in CI, JSON, quiet mode, or serious errors
- [ ] benchmarks compare against google-colab-cli and manual workflows
- [ ] README claims match `docs/claims-ledger.md`

## Current Priorities

1. Get the command tree boring and stable
2. Make fs sync useful before making it fancy
3. Keep continuation honest
4. Make Slurp readable
5. Make status checks good enough that users actually run them
6. Benchmark before bragging
7. Prune features that do not help the main workflow

## Non-Goals

- free-tier cluster mode
- account rotation to bypass limits
- live Python memory migration
- plugin marketplace
- giant TUI
- magic agent autonomy
- official Google replacement claim

## Risks

- Colab APIs and browser/runtime behaviour can change
- network benchmarks are noisy
- continuation can be oversold if wording gets sloppy
- multi-account support can accidentally look like quota bypassing
- file sync can become complicated fast

## Next Implementation Pass

- clean up command aliases
- make `status quick` tighter
- make `fs changed` compare against a real remote/cache manifest
- finish compact output style
- finish `slurp explain`
- add redacted bug report output checks
- run first real competitor benchmark pass
- update claims ledger

## How I’ll Know This Is Useful

- first run works without reading a novel
- dry-run output is clear
- failed auth tells the next command
- no-op sync sends almost nothing
- resume does not lie about what it can restore
- docs are short enough to actually read
