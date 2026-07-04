# Colab Transport Audit

Scope: `google-colab-cli`, `colabtools`, and current Rust `cocli` transport paths for assignment, runtime proxying, Jupyter REST/WebSocket, `/colab/tty`, Contents API, logs, and kernel controls.

## Sources Read

- `google-colab-cli/src/colab_cli/client.py`: Colab front-door `/tun/m` REST, XSSI stripping, XSRF assignment flow, keep-alive.
- `google-colab-cli/src/colab_cli/runtime.py`: `jupyter_kernel_client` over Colab runtime proxy, kernel execution, `colab_request` interception.
- `google-colab-cli/src/colab_cli/console.py`: `/colab/tty` WebSocket raw terminal.
- `google-colab-cli/src/colab_cli/contents.py`: Jupyter Contents API file transfer.
- `google-colab-cli/src/colab_cli/commands/automation.py`: Drive auth propagation through `colab_request`.
- `src/cocli/session/client.rs`: Rust Colab client, assignment, Jupyter REST, Contents API.
- `src/cocli/exec/runner.rs`: Rust raw Jupyter WebSocket execution and `/colab/tty`.
- `src/cocli/cli/dispatch.rs`: command wiring, Drive mount/status, logs, kernel controls.

## Front-Door Colab API

Real flow:

1. All assignment/control requests go to `https://colab.research.google.com/tun/m/...`.
2. Requests include `Authorization: Bearer <Google access token>`, `Accept: application/json`, and `X-Colab-Client-Agent`.
3. Colab front-door responses may start with XSSI prefix `)]}'\n`; clients strip it before JSON parsing.
4. Requests against the Colab domain carry `authuser=0`.
5. Tunnel-intercepted paths need `X-Colab-Tunnel: Google`.

`google-colab-cli` implements this in `Client._issue_request()` and constants around `TUN_ENDPOINT = "/tun/m"` (`google-colab-cli/src/colab_cli/client.py:28`, `google-colab-cli/src/colab_cli/client.py:135`, `google-colab-cli/src/colab_cli/client.py:163`).

`cocli` mirrors it in `ColabClient::colab_request()`, `colab_url()`, and `strip_xssi()` (`src/cocli/session/client.rs:24`, `src/cocli/session/client.rs:599`, `src/cocli/session/client.rs:635`).

Implement:

- Keep `/tun/m` and `authuser=0`; do not invent a separate backend.
- Keep XSSI stripping at the shared parse boundary.
- Keep redacted verbose logging for method/path/status only.

Do not fake:

- Do not fabricate runtime state from local session files. Local state is only a cache; refresh against `/tun/m/assignments` when users ask for live state.

## Assign, Refresh, Unassign

Assign:

1. Generate a notebook hash as UUID with `-` replaced by `_` and padded to 44 chars with `.`.
2. `GET /tun/m/assign?authuser=0&nbh=<hash>[&variant=GPU|TPU][&accelerator=T4...][&shape=hm]`.
3. If response already contains `endpoint`, use it as an existing assignment.
4. Otherwise parse `token` from the GET response and `POST` the same URL with `X-Goog-Colab-Token: <token>` and empty body.
5. Persist `endpoint`, `runtimeProxyInfo.url`, `runtimeProxyInfo.token`, and token expiry.

`google-colab-cli` implements GET then POST in `assign()`, `_get_assignment()`, and `_post_assignment()` (`google-colab-cli/src/colab_cli/client.py:226`). `cocli` does the same in `ColabClient::assign()` and adds High-RAM `shape=hm` (`src/cocli/session/client.rs:87`, `src/cocli/session/client.rs:770`).

Refresh:

- `cocli` calls `GET /tun/m/runtime-proxy-token?authuser=0&endpoint=<endpoint>&port=8080` with `X-Colab-Tunnel: Google` and updates the stored proxy token (`src/cocli/session/client.rs:147`).

Unassign:

1. `GET /tun/m/unassign/<endpoint>` to get an XSRF token.
2. `POST /tun/m/unassign/<endpoint>` with `X-Goog-Colab-Token`.
3. `cocli` deletes Jupyter sessions first via the runtime tunnel, then unassigns (`src/cocli/session/commands.rs:121`).

Implement:

- Treat assignment 412 as "too many assignments".
- Keep quota/denylist outcome handling from Rust models.
- Refresh proxy tokens before long file/shell operations.

Do not fake:

- Do not mark local sessions stopped until unassign succeeds, except when explicitly reconciling stale local records against server state.

## Jupyter REST

Two equivalent paths exist:

- Tunnel path through Colab front-door: `/tun/m/<endpoint>/api/...` with Google bearer token and `X-Colab-Tunnel: Google`.
- Direct runtime proxy URL from `runtimeProxyInfo.url`: `<proxy_url>/api/...` with `X-Colab-Runtime-Proxy-Token`.

`google-colab-cli` mostly uses the proxy URL through `jupyter_kernel_client` and `ContentsClient`. `cocli` uses both:

- `/tun/m/<endpoint>/api/sessions` for session discovery (`src/cocli/session/client.rs:156`).
- Direct proxy URL for `/api/kernels`, `/api/kernelspecs`, `/api/sessions`, `/api/terminals`, and `/api/contents` (`src/cocli/session/client.rs:164`, `src/cocli/session/client.rs:183`, `src/cocli/session/client.rs:202`, `src/cocli/session/client.rs:266`, `src/cocli/session/client.rs:355`).

Implement:

- Prefer direct proxy URL for Jupyter API calls once a fresh proxy token is known.
- Use tunnel `/api/sessions` when validating whether an assignment has a browser-created kernel session.
- Send `X-Colab-Runtime-Proxy-Token` as a header on direct proxy REST calls.

Do not fake:

- Do not assume a kernel exists just because an assignment exists. Drive and REPL need a Jupyter session with a kernel; plain runtime assignment is not enough.

## Kernel WebSocket

`google-colab-cli` delegates to `jupyter_kernel_client.KernelClient`, passing:

- `server_url=<runtimeProxyInfo.url>`
- `token=<runtimeProxyInfo.token>`
- extra query param `colab-runtime-proxy-token=<token>`
- headers `X-Colab-Client-Agent` and `X-Colab-Runtime-Proxy-Token`

It sets `_own_kernel = False` so closing the CLI does not delete the remote kernel (`google-colab-cli/src/colab_cli/runtime.py:98`, `google-colab-cli/src/colab_cli/runtime.py:106`, `google-colab-cli/src/colab_cli/runtime.py:116`).

`cocli` hand-builds Jupyter protocol messages over `tokio_tungstenite`:

- Discovers session and kernel IDs.
- Connects to the kernel websocket.
- Sends `execute_request` with `msg_type = "execute_request"`, channel `shell`, and `allow_stdin = false`.
- Collects messages by matching `parent_header.msg_id`.
- Stops when a `status` message reports `execution_state = "idle"` (`src/cocli/exec/runner.rs:70`).

Implement:

- Keep raw WebSocket execution small and protocol-correct.
- Add stdin/input-reply support only when needed for a real workflow; Drive approval should be handled through Colab `colab_request`, not generic `input()`.

Do not fake:

- Do not report execution complete before the matching parent message reaches idle.
- Do not close or delete kernels implicitly when a client disconnects.

## `/colab/tty`

Real terminal flow:

1. Convert proxy URL scheme to `wss://` or `ws://`.
2. Connect to `/colab/tty?colab-runtime-proxy-token=<token>`.
3. Send terminal size as JSON `{ "cols": N, "rows": N }`.
4. Send stdin as JSON `{ "data": "..." }`.
5. Read JSON frames containing `data` and write raw ANSI text to stdout.
6. For piped stdin, send `exit\n`, wait briefly for tail output, then close.

`google-colab-cli` implements this in `console.py` (`google-colab-cli/src/colab_cli/console.py:87`). `cocli` implements it in `run_shell()` and helper functions (`src/cocli/exec/runner.rs:226`, `src/cocli/exec/runner.rs:406`, `src/cocli/exec/runner.rs:421`).

`cocli` also uses standard Jupyter terminals:

- `POST /api/terminals`
- WebSocket `/terminals/websocket/<name>`
- frames like `["stdin", "..."]` and `["set_size", rows, cols]`

Those are for command capture/TUI helpers, not the Colab-specific shell (`src/cocli/exec/runner.rs:449`).

Implement:

- Keep `/colab/tty` for the user shell because it matches Colab's raw PTY behavior.
- Keep Jupyter terminals for bounded command capture where cleanup matters.

Do not fake:

- Do not implement shell by sending `!cmd` to the kernel. That loses PTY behavior, signal handling, and shell state.

## Contents API

`google-colab-cli` uses direct proxy `GET|PUT|DELETE <proxy_url>/api/contents/<path>` with query param `colab-runtime-proxy-token` (`google-colab-cli/src/colab_cli/contents.py:20`).

`cocli` uses the same Jupyter Contents model with the proxy token as a header:

- Upload: `PUT /api/contents/<path>` with JSON `{ "type": "file", "format": "base64", "content": "..." }`.
- Stat: `GET /api/contents/<path>?content=0`.
- List: `GET /api/contents/<path>`.
- Download: `GET /api/contents/<path>?type=file&format=base64`, then decode `content`.
- Encode path segments, not slashes (`src/cocli/session/client.rs:355`, `src/cocli/session/client.rs:456`, `src/cocli/session/client.rs:481`, `src/cocli/session/client.rs:525`, `src/cocli/session/client.rs:724`).

Implement:

- Continue using Jupyter Contents API for `/content` file operations.
- Refresh proxy token during recursive/long downloads.
- Keep base64 payload handling; do not switch to ad hoc shell `cat` for file transfer.

Do not fake:

- Do not claim remote file sync is durable without testing live Contents timestamp/hash behavior.

## Kernel Controls

Jupyter REST controls:

- List kernels: `GET /api/kernels`.
- Kernelspecs: `GET /api/kernelspecs`.
- Start kernel: `POST /api/kernels` with `{ "name": "<spec>" }`.
- Interrupt/restart: `POST /api/kernels/<id>/<action>`.
- Shutdown: `DELETE /api/kernels/<id>`.

These are implemented in `cocli` client methods (`src/cocli/session/client.rs:164`, `src/cocli/session/client.rs:183`, `src/cocli/session/client.rs:202`, `src/cocli/session/client.rs:223`, `src/cocli/session/client.rs:317`) and command dispatch (`src/cocli/cli/dispatch.rs:2240`).

Implement:

- Confirmation-gate restart/shutdown.
- Cache selected kernel ID/name locally only as a convenience; refresh from Jupyter APIs when acting.

Do not fake:

- Do not show cached selected kernel as live status unless it was refreshed or verified.

## Logs

`google-colab-cli` has a local `HistoryLogger` and records events around execution/automation. It is not a remote Colab log API.

`cocli` currently exposes `session logs`, but returns an explicit empty result: `available: false`, empty logs, and a note that no persisted stream exists (`src/cocli/cli/dispatch.rs:986`).

Implement:

- Keep local command history/log export only after cocli actually persists executions.
- Consider remote stdout/stderr streaming as runtime output, not "session logs".

Do not fake:

- Do not invent remote logs. If no cocli log stream exists, say so exactly.
