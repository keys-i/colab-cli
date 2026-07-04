# Runtime Endpoint Root Cause

Date: 2026-07-04

## Symptom

After a fresh `colab session new`, these commands could fail immediately:

```text
colab run repl
colab fs drive mount
colab session kernel current
```

The failure was reported as a stale or timed-out Jupyter sessions endpoint even
though assignment had just succeeded.

## Cause

The assignment flow stored both:

- Colab tunnel endpoint: `endpoint`
- Runtime proxy connection: `proxy_url` and `proxy_token`

Several runtime commands used:

```text
https://colab.research.google.com/tun/m/<endpoint>/api/sessions?authuser=0
```

as their first preflight. Local google-colab-cli behavior uses the stored runtime
proxy URL/token for Jupyter runtime APIs instead.

That made a healthy runtime look stale when the tunnel sessions route timed out
or did not behave like the runtime proxy API.

## Fix

Added:

```rust
ColabClient::list_sessions(proxy_url, proxy_token)
```

It calls:

```text
GET <proxy_url>/api/sessions
X-Colab-Runtime-Proxy-Token: <proxy_token>
X-Colab-Client-Agent: vscode
Accept: application/json
```

Updated runtime callers to prefer this proxy path:

- `run repl`
- `fs drive mount`
- `session kernel current/list/select` through kernel view loading
- `session repair`
- `run pkg` active-kernel selection
- shared kernel-cell execution helper

The older tunnel sessions call remains only as fallback or cleanup.

## Classification Rule

- Timeout stays `runtime_endpoint_timeout`.
- 401/403 is auth/runtime credential failure.
- 404/gone is stale endpoint.
- A fresh session is not marked stale only because the tunnel sessions path is
  unavailable.

## Not Claimed

This fix corrects endpoint selection and mocked/local routing. Live REPL, Drive,
kernel, and shell behavior still require the live smoke suite before claiming
real Colab transport success.
