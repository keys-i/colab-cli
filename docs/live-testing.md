# Live Testing

Live tests are manual because they need Google auth, a real Colab runtime, and sometimes browser approval.

```sh
COLAB_CLI_LIVE=1 ./scripts/live-smoke.sh
```

Secrets bridge smoke is separate and must use a disposable test value:

```sh
COLAB_CLI_LIVE=1 COLAB_CLI_SECRET_TEST=1 COLAB_CLI_TEST_SECRET=not-real-test-secret ./scripts/live-secrets-smoke.sh
```

The script writes a short report to:

```text
target/live-smoke.md
```

## What It Checks

- `session list`
- a small Python command
- `status runtime --all`
- `fs ls /content`
- `fs drive status`
- `fs drive mount --timeout 180`
- `ai tools list`
- `ai tools list --json`
- `status check`

The secrets smoke checks that `userdata.get("COLAB_CLI_TEST_SECRET")` returns a
length and that the raw value is not printed.

## Drive Approval

Drive mount can ask for browser approval. In an interactive shell the script prints the command to open the session URL and waits for Enter. In CI or any non-interactive shell, it marks Drive mount as manual and continues.

The script must never hang forever and must not stop or delete sessions it did not create.

## Not Covered

The live smoke is not a full Colab compatibility suite. It does not prove long-running notebook execution, Drive behaviour in Colab Enterprise, assignment retries under real 503/429 responses, secrets bridge stability, or continuation replay under real failure. Those need focused manual tests.
