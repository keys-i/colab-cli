# Auth and Drive Audit

Scope: OAuth, ADC, token storage, in-runtime `google.colab.auth`, Drive mount, credentials propagation, and what `colab` should implement or refuse to fake.

## Host OAuth

`google-colab-cli` supports two host auth providers:

- `oauth2`: Installed-app OAuth using public scopes.
- `adc`: Application Default Credentials via `google.auth.default()`.

Its public scopes are:

- `openid`
- `https://www.googleapis.com/auth/userinfo.profile`
- `https://www.googleapis.com/auth/userinfo.email`
- `https://www.googleapis.com/auth/cloud-platform`
- `https://www.googleapis.com/auth/colaboratory`
- `https://www.googleapis.com/auth/drive.file`

OAuth2 uses a remote copy-paste flow with `https://sdk.cloud.google.com/applicationdefaultauthcode.html`, not OOB and not localhost (`google-colab-cli/src/colab_cli/auth.py:32`, `google-colab-cli/src/colab_cli/auth.py:57`).

`colab` currently uses browser localhost OAuth with PKCE:

1. Bind `127.0.0.1:0`.
2. Build auth URL with nonce and code challenge.
3. Open browser or print URL.
4. Accept one redirect.
5. Validate nonce.
6. Exchange code for access/refresh tokens.
7. Store access token, refresh token, and user info (`src/cocli/auth/oauth.rs:23`, `src/cocli/auth/oauth.rs:36`, `src/cocli/auth/oauth.rs:104`, `src/cocli/auth/oauth.rs:169`).

`colab` required scopes are currently only:

- `profile`
- `email`
- `https://www.googleapis.com/auth/colaboratory`

Implement:

- Keep PKCE and nonce validation.
- Add Drive scope only when colab implements Drive credentials propagation itself.
- If remote/headless login is a target, add a copy-paste flow rather than relying only on localhost.

Do not fake:

- Do not claim Drive host authorization works until the OAuth scopes and credentials-propagation endpoint are implemented and live-tested.

## Token Storage

`colab` stores credentials under the platform local data dir as `colab/credentials.json` with mode `0600` on Unix. It stores:

- refresh token
- access token plus expiry
- account email/name

Access tokens are cached in memory until five minutes before expiry, then refreshed with the stored refresh token (`src/cocli/auth/storage.rs:14`, `src/cocli/auth/storage.rs:38`, `src/cocli/auth/mod.rs:13`, `src/cocli/auth/mod.rs:24`).

Implement:

- Keep writes private and atomic.
- Redact access tokens, refresh tokens, proxy tokens, XSRF tokens, bearer headers, and home paths in logs.

Do not fake:

- Do not print raw credentials in `--json`, bug reports, verbose logs, or exported auth profiles.

## ADC

`google-colab-cli` ADC path calls `google.auth.default(scopes=PUBLIC_SCOPES)`, handles scope requirements, suppresses the irrelevant Cloud SDK quota-project warning, and wraps credentials in an `AuthorizedSession` (`google-colab-cli/src/colab_cli/auth.py:133`).

`colab` currently does not use ADC credentials for Colab API calls. `auth login --method adc` only checks whether an ADC file exists at `GOOGLE_APPLICATION_CREDENTIALS` or `~/.config/gcloud/application_default_credentials.json` and prints setup guidance (`src/cocli/cli/dispatch.rs:884`).

Implement:

- Either implement true ADC token acquisition for Colab requests or keep ADC labeled as detection-only.
- True ADC needs the same Colab-required scopes and must feed the existing `ColabClient` bearer-token callback.

Do not fake:

- Do not mark a user authenticated for Colab just because an ADC file exists.
- Do not auto-switch accounts or use fallback profiles to bypass quota or limits.

## In-Runtime `google.colab.auth.authenticate_user()`

This is different from host CLI auth.

`colabtools/google/colab/auth.py` runs inside a Colab kernel. It:

1. Checks whether ADC exists and is valid inside the VM.
2. If project ID is supplied, configures gcloud project and quota env vars.
3. For ephemeral auth, sends `colab_request` of type `request_auth` with `authType = auth_user_ephemeral`.
4. Otherwise runs `gcloud auth login --enable-gdrive-access --no-launch-browser`, writes ADC into `/content/.adc/adc.json`, and sets `GOOGLE_APPLICATION_CREDENTIALS`.

Implement:

- Treat runtime auth as kernel automation, not host auth.
- If exposing it, execute `from google.colab import auth; auth.authenticate_user()` in the active Colab kernel and handle its `colab_request` path.

Do not fake:

- Do not copy host refresh tokens into the VM.
- Do not claim host OAuth automatically authenticates Python client libraries inside the runtime.

## Drive Mount Real Flow

`google.colab.drive.mount(path)` runs inside the Colab VM. It is not a host mount.

`colabtools/google/colab/drive.py` does this:

1. Refuses non-Colab environments by checking `/var/colab/hostname`.
2. Refuses mountpoints containing spaces and unsupported Enterprise/MP environments.
3. Chooses metadata server address from `TBE_EPHEM_CREDS_ADDR` for ephemeral auth.
4. Sends `google.colab._message.blocking_request("request_auth", {"authType": "dfs_ephemeral"})`.
5. Starts DriveFS binary from `<root>/opt/google/drive/drive`.
6. Passes `--metadata_server_auth_uri=<addr>/computeMetadata/v1`.
7. Waits until the mountpoint contains files and prints `Mounted at <path>`.
8. Starts log filtering and optional directory prefetcher (`colabtools/google/colab/drive.py:95`, `colabtools/google/colab/drive.py:113`, `colabtools/google/colab/drive.py:135`, `colabtools/google/colab/drive.py:222`, `colabtools/google/colab/drive.py:246`).

Unmount uses DriveFS `--push_changes_and_quit` and checks `/bin/mount` for `type fuse.drive` (`colabtools/google/colab/drive.py:69`).

Implement:

- Drive mount must run through a Colab kernel session.
- Use `from google.colab import drive; drive.mount("/content/drive", force_remount=False)`.
- Keep preflight that verifies an IPython kernel context.
- Keep status as an actual remote probe of `/content/drive`.

Do not fake:

- Do not mount Google Drive on the host.
- Do not shell-create `/content/drive` and call it mounted.
- Do not treat a plain assigned runtime without browser-created kernel session as mount-capable.

## Drive Credentials Propagation Gap

Drive auth is the main missing real-transport piece.

`google-colab-cli` handles DriveFS auth by intercepting kernel WebSocket messages of type `colab_request`. When the request content has `authType = dfs_ephemeral`, it:

1. Reads `metadata.colab_msg_id`.
2. Calls `GET /tun/m/credentials-propagation/<endpoint>` with params:
   - `authuser=0`
   - `authtype=dfs_ephemeral`
   - `version=2`
   - `dryrun=true`
   - `propagate=true`
   - `record=false`
3. Extracts returned `token`.
4. Calls `POST` to the same endpoint with `x-goog-colab-token: <token>` and multipart `file_id=empty.ipynb`.
5. If response says not successful, prints `unauthorized_redirect_uri` and waits for user approval.
6. Calls `POST` again with `dryrun=false`.
7. Sends a Jupyter `input_reply` on stdin containing `{ "type": "colab_reply", "colab_msg_id": <msg_id> }`.

See `google-colab-cli/src/colab_cli/commands/automation.py:55`.

`colab` currently executes the Drive mount cell and classifies `request_auth`/`blocking_request` output as `drive_browser_approval_required`, telling the user to open the browser session and retry (`src/cocli/cli/dispatch.rs:2919`, `src/cocli/cli/dispatch.rs:3480`, `src/cocli/cli/dispatch.rs:3529`).

Implement:

- Add `colab_request` interception to Rust kernel WebSocket execution before claiming first-class CLI Drive mount.
- Implement `/tun/m/credentials-propagation/<endpoint>` GET/POST with XSRF token.
- Reply on the kernel stdin channel with the matching `colab_msg_id`.
- Keep the current browser-open fallback as a fallback, not as the claimed complete path.

Do not fake:

- Do not swallow `request_auth` and report success.
- Do not assume browser approval happened unless credentials propagation returns success or the subsequent remote status probe confirms a mounted Drive.

## Current `colab` Drive Behavior

`colab fs drive mount` currently:

1. Loads selected local session.
2. Refreshes proxy token if needed.
3. Validates endpoint and proxy URL.
4. Calls `/tun/m/<endpoint>/api/sessions`.
5. Selects a session with a kernel.
6. Probes existing mount through a temporary Jupyter terminal command.
7. Optionally opens the browser session.
8. Executes a kernel preflight cell.
9. Executes `from google.colab import drive; drive.mount(...)`.
10. Classifies timeout/auth/unsupported errors.
11. Probes `/content/drive` again.

This is the right shape, but incomplete without DriveFS credentials propagation. It should continue saying "Drive needs browser approval" when the `colab_request` path appears.

Implement:

- Keep staged errors and retry around `/api/sessions`.
- Keep `--dry-run` honest: it should say "would execute" and "needs kernel".

Do not fake:

- Do not mark mount successful based only on starting the cell.
- Do not treat timeout during approval as success unless status probe confirms mounted.

## Logs and Audit Trail

`google-colab-cli` records local history events for automation, Drive auth needed/success, stdin, and outputs. `colab` has debug logging and a `session logs` placeholder that explicitly says no persisted stream is available.

Implement:

- If adding persisted logs, record colab commands, kernel execution requests, selected session, and summarized outputs with redaction.
- Keep Drive auth events as local audit entries, not remote logs.

Do not fake:

- Do not invent Colab server logs.
- Do not include auth URLs or tokens in logs unless redacted.

## Minimum Honest Roadmap

1. Keep existing assignment, Jupyter REST, Contents API, kernel control, and `/colab/tty` transports.
2. Implement true ADC or rename/keep it as detection-only.
3. Add Rust `colab_request` interception for `dfs_ephemeral`.
4. Add credentials propagation endpoint support.
5. Only then describe `fs drive mount` as fully CLI-native.
