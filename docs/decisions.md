# Decisions

## Workspace First

The project is now a Cargo workspace with publishable crates. The existing working Colab HTTP code stayed in `colab-cli` until live integration tests justify moving it.

## Enum Tool Registry

Built-in tools use enum dispatch. Add trait objects when external plugin loading needs them.

## Sync Dry-Run First

`cocli-fs` implements local manifests and diff planning. `fs sync` writes no remote data unless future work verifies remote timestamp and hash semantics against Colab Contents API responses.

## Continuation Is Honest

Continuation is checkpoint/replay, not process transfer.

## No Unsafe

The only prior unsafe string slice was replaced with a safe slice after an ASCII prefix check.
