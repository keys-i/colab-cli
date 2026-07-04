# Google Transport Parity

Date: 2026-07-04

## Sources Checked

- installed `colab` help, with sandbox-safe `HOME`/`XDG_CONFIG_HOME`
- local `google-colab-cli/` source and docs
- local `colabtools/` reference files

No Google code is vendored or copied.

## Reference Behavior

| Google command | Cocli command | Transport |
|---|---|---|
| `colab repl` | `colab run repl` | Jupyter kernel messaging with a local line editor. |
| `colab console` | `colab run shell` | Colab `/colab/tty` websocket, not Jupyter `/api/terminals` as primary path. |
| `colab exec` | `colab run py/script/notebook` | Jupyter kernel execution. |
| `colab drivemount` | `colab fs drive mount` | Kernel execution plus Colab credential propagation. |
| `colab log` | `colab log` | Local JSONL history/export. |
| `colab restart-kernel` | `colab session kernel restart` | Jupyter kernel control API. |
| `colab update` | `colab update` | Explicit update check/install. |
| `colab pay` | `colab pay` | Opens Colab billing page. |
| `colab version` | `colab version` | Local version output. |

## Transport Split

Tunnel endpoints are for assignment, unassignment, keep-alive, browser/runtime
attachment, and Drive credential propagation:

```text
/tun/m/assignments
/tun/m/assign
/tun/m/unassign/<endpoint>
/tun/m/<endpoint>/keep-alive/
/tun/m/credentials-propagation/<endpoint>
```

Runtime work should use the assigned `runtime_proxy_info.url` with
`X-Colab-Runtime-Proxy-Token`:

```text
/api/sessions
/api/kernels
/api/kernelspecs
/api/contents
/colab/tty
```

## Cocli Fix Applied

`ColabClient::list_sessions(proxy_url, proxy_token)` now checks runtime
sessions through the proxy path first. REPL, Drive mount, kernel current, session
repair, and package routing use that helper before falling back to the older
tunnel sessions path.

This addresses the fresh-session failure mode where assignment succeeds but
`/tun/m/<endpoint>/api/sessions` immediately times out or looks stale.

## Still Live-Tested Separately

The local code now uses the right transport shape, but live REPL, shell, Drive
mount, and credential propagation still require `COLAB_CLI_LIVE=1` smoke tests
before claiming end-to-end live parity.
