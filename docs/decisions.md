# Decisions

## One Crate

The project is one publishable package. Checked against the reference CLI shape; the public API is not stable enough to justify crate splits.

Crates can be extracted later when an external user or plugin proves the boundary.

## Enum Tool Registry

Built-in tools use enum dispatch. Add trait objects only when a real external plugin exists.

## Sync Dry-Run First

`fs sync` plans local changes. It writes no remote data until remote timestamp/hash behavior is tested against live Colab Contents API responses.

## Continuation Is Honest

Continuation is checkpoint/replay, not live Python process transfer.

## No Unsafe

Unsafe code is forbidden by package lints.
