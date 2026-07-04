use std::future::Future;
use std::io::{self, IsTerminal, Read, Write};
use std::pin::Pin;
use std::sync::Arc;

use crossterm::terminal;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;

use crate::cocli::error::{ColabError, Result};
use crate::cocli::kernel::KernelInfoSummary;
use crate::cocli::session::client::ColabClient;
use crate::cocli::session::model::Session;
use crate::cocli::session::store::StoredServer;

// async refresher used by long-running shells to rotate the proxy token.
// returns the new StoredServer so reconnect can pick up the rotated value.
pub type TokenRefresher =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<StoredServer>> + Send>> + Send + Sync>;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CellOutput {
    pub stdout: String,
    pub stderr: String,
    pub error_name: Option<String>,
    pub error_value: Option<String>,
    pub traceback: Vec<String>,
    pub timed_out: bool,
}

impl CellOutput {
    pub fn raw_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.stdout);
        out.push_str(&self.stderr);
        if let Some(name) = &self.error_name {
            out.push_str(name);
            out.push('\n');
        }
        if let Some(value) = &self.error_value {
            out.push_str(value);
            out.push('\n');
        }
        for line in &self.traceback {
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}

pub async fn execute_colab_cell(
    client: &ColabClient,
    server: &StoredServer,
    code: &str,
    timeout: std::time::Duration,
) -> Result<CellOutput> {
    let sessions = client.list_sessions_via_tunnel(&server.endpoint).await?;
    let Some(session) = sessions.iter().find(|s| s.kernel.is_some()) else {
        return Err(ColabError::drive(
            "drive_kernel_context_required",
            "Drive mount needs a Colab kernel session, not a plain Python process",
            Some("colab-cli session url --open"),
            None,
        ));
    };
    execute_colab_cell_in_session(client, server, session, code, timeout).await
}

pub async fn execute_colab_cell_in_session(
    _client: &ColabClient,
    server: &StoredServer,
    session: &Session,
    code: &str,
    timeout: std::time::Duration,
) -> Result<CellOutput> {
    let kernel_id = session
        .kernel
        .as_ref()
        .map(|kernel| kernel.id.as_str())
        .ok_or_else(|| {
            ColabError::drive(
                "drive_kernel_context_required",
                "Drive mount needs a Colab kernel session, not a plain Python process",
                Some("colab-cli session url --open"),
                None,
            )
        })?;

    let ws_url = kernel_ws_url(&server.proxy_url, kernel_id, &session.id);
    let request = build_ws_request(&ws_url, &server.proxy_token)?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| ColabError::oauth(format!("Kernel WebSocket connect failed: {e}")))?;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    let msg_id = uuid::Uuid::new_v4().to_string();
    let execute = serde_json::json!({
        "header": {
            "msg_id": msg_id,
            "username": "colab-cli",
            "session": session.id,
            "date": chrono::Utc::now().to_rfc3339(),
            "msg_type": "execute_request",
            "version": "5.3"
        },
        "parent_header": {},
        "metadata": {},
        "content": {
            "code": code,
            "silent": false,
            "store_history": true,
            "user_expressions": {},
            "allow_stdin": false,
            "stop_on_error": true
        },
        "channel": "shell"
    });
    ws_write
        .send(tungstenite::Message::Text(execute.to_string().into()))
        .await
        .map_err(|e| ColabError::oauth(format!("Kernel WebSocket send: {e}")))?;

    let deadline = tokio::time::Instant::now() + timeout;
    let mut out = CellOutput::default();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            out.timed_out = true;
            return Ok(out);
        }

        let msg = tokio::time::timeout(remaining, ws_read.next()).await;
        let text = match msg {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => text,
            Ok(Some(Ok(tungstenite::Message::Close(_)))) | Ok(None) => return Ok(out),
            Ok(Some(Err(e))) => return Err(ColabError::oauth(format!("Kernel WebSocket: {e}"))),
            Err(_) => {
                out.timed_out = true;
                return Ok(out);
            }
            _ => continue,
        };

        let Ok(value) = serde_json::from_str::<serde_json::Value>(text.as_ref()) else {
            continue;
        };
        if value
            .pointer("/parent_header/msg_id")
            .and_then(|v| v.as_str())
            != Some(msg_id.as_str())
        {
            continue;
        }
        collect_cell_message(&value, &mut out);
        if value.pointer("/header/msg_type").and_then(|v| v.as_str()) == Some("status")
            && value
                .pointer("/content/execution_state")
                .and_then(|v| v.as_str())
                == Some("idle")
        {
            return Ok(out);
        }
    }
}

pub async fn kernel_info(
    server: &StoredServer,
    session: &Session,
    timeout: std::time::Duration,
) -> Result<KernelInfoSummary> {
    let kernel_id = session
        .kernel
        .as_ref()
        .map(|kernel| kernel.id.as_str())
        .ok_or_else(|| ColabError::config("kernel unavailable"))?;
    let ws_url = kernel_ws_url(&server.proxy_url, kernel_id, &session.id);
    let request = build_ws_request(&ws_url, &server.proxy_token)?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| ColabError::oauth(format!("Kernel WebSocket connect failed: {e}")))?;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    let msg_id = uuid::Uuid::new_v4().to_string();
    let request = serde_json::json!({
        "header": {
            "msg_id": msg_id,
            "username": "colab-cli",
            "session": session.id,
            "date": chrono::Utc::now().to_rfc3339(),
            "msg_type": "kernel_info_request",
            "version": "5.3"
        },
        "parent_header": {},
        "metadata": {},
        "content": {},
        "channel": "shell"
    });
    ws_write
        .send(tungstenite::Message::Text(request.to_string().into()))
        .await
        .map_err(|e| ColabError::oauth(format!("Kernel WebSocket send: {e}")))?;

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(KernelInfoSummary::unknown());
        }
        let msg = tokio::time::timeout(remaining, ws_read.next()).await;
        let text = match msg {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => text,
            Ok(Some(Ok(tungstenite::Message::Close(_)))) | Ok(None) => {
                return Ok(KernelInfoSummary::unknown());
            }
            Ok(Some(Err(e))) => return Err(ColabError::oauth(format!("Kernel WebSocket: {e}"))),
            Err(_) => return Ok(KernelInfoSummary::unknown()),
            _ => continue,
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text.as_ref()) else {
            continue;
        };
        if value
            .pointer("/parent_header/msg_id")
            .and_then(|v| v.as_str())
            != Some(msg_id.as_str())
        {
            continue;
        }
        if value.pointer("/header/msg_type").and_then(|v| v.as_str()) == Some("kernel_info_reply") {
            return Ok(KernelInfoSummary::from_language_info(
                value
                    .pointer("/content/language_info/name")
                    .and_then(|v| v.as_str()),
                value
                    .pointer("/content/language_info/version")
                    .and_then(|v| v.as_str()),
            ));
        }
    }
}

pub async fn run_shell(
    client: &ColabClient,
    server: &StoredServer,
    initial_command: Option<&str>,
    refresher: Option<TokenRefresher>,
) -> Result<()> {
    let ws_url = colab_tty_ws_url(&server.proxy_url, &server.proxy_token)?;
    crate::cocli::debug::debug1("run.shell transport=colab_tty");
    crate::cocli::debug::debug1("run.shell websocket connecting path=/colab/tty");
    crate::cocli::debug::debug3(format!(
        "run.shell websocket url={}",
        crate::cocli::debug::sanitize_url(&ws_url)
    ));
    let request = build_ws_request(&ws_url, &server.proxy_token)?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(shell_unavailable_error)?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    if let Ok((cols, rows)) = terminal::size() {
        let _ = send_colab_tty_resize(&mut ws_write, rows, cols).await;
    }

    if let Some(cmd) = initial_command {
        let _ = send_colab_tty_input(&mut ws_write, &format!("{cmd}\n")).await;
    }

    // ping every 4min to keep the runtime warm and rotate the proxy token.
    // the open ws stays pinned to its original token, but any reconnect or
    // sibling http call via the proxy would 401 once the token expired.
    let keepalive_client = client.clone();
    let keepalive_endpoint = server.endpoint.clone();
    let keepalive_refresher = refresher.clone();
    let keepalive_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(4 * 60));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Some(refresher) = keepalive_refresher.as_ref() {
                // we don't need the new StoredServer here — the side-effect
                // (rotated token in storage) is what matters
                let _ = (refresher)().await;
            }
            let _ = keepalive_client.send_keep_alive(&keepalive_endpoint).await;
        }
    });
    let _keepalive_guard = AbortOnDrop(keepalive_handle);

    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        if !input.is_empty() {
            send_colab_tty_input(&mut ws_write, &input).await?;
        }
        send_colab_tty_input(&mut ws_write, "exit\n").await?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, ws_read.next()).await {
                Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                    if let Some(data) = parse_colab_tty_frame(text.as_ref()) {
                        let mut stdout = io::stdout().lock();
                        let _ = stdout.write_all(data.as_bytes());
                        let _ = stdout.flush();
                    }
                }
                Ok(Some(Ok(tungstenite::Message::Binary(data)))) => {
                    let mut stdout = io::stdout().lock();
                    let _ = stdout.write_all(&data);
                    let _ = stdout.flush();
                }
                Ok(Some(Ok(tungstenite::Message::Close(_)))) | Ok(None) => break,
                _ => {}
            }
        }
        let _ = ws_write.close().await;
        return Ok(());
    }

    terminal::enable_raw_mode().map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
    let _raw_guard = RawModeGuard;

    #[derive(Debug)]
    enum ShellOut {
        Stdin(Vec<u8>),
        Resize(u16, u16),
    }

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<ShellOut>(64);
    let resize_tx = stdin_tx.clone();

    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buf = [0u8; 4096];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if stdin_tx
                        .blocking_send(ShellOut::Stdin(buf[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let resize_handle = tokio::spawn(async move {
        let mut last = terminal::size().unwrap_or((80, 24));
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
        tick.tick().await;
        loop {
            tick.tick().await;
            let cur = terminal::size().unwrap_or(last);
            if cur != last
                && resize_tx
                    .send(ShellOut::Resize(cur.1, cur.0))
                    .await
                    .is_err()
            {
                return;
            }
            last = cur;
        }
    });
    let _resize_guard = AbortOnDrop(resize_handle);

    loop {
        tokio::select! {
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        if let Some(data) = parse_colab_tty_frame(text.as_ref()) {
                            let mut stdout = io::stdout().lock();
                            let _ = stdout.write_all(data.as_bytes());
                            let _ = stdout.flush();
                        }
                    }
                    Some(Ok(tungstenite::Message::Binary(data))) => {
                        let mut stdout = io::stdout().lock();
                        let _ = stdout.write_all(&data);
                        let _ = stdout.flush();
                    }
                    Some(Ok(tungstenite::Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            data = stdin_rx.recv() => {
                match data {
                    Some(ShellOut::Stdin(bytes)) => {
                        let text = String::from_utf8_lossy(&bytes);
                        if send_colab_tty_input(&mut ws_write, &text).await.is_err() {
                            break;
                        }
                    }
                    Some(ShellOut::Resize(rows, cols)) => {
                        let _ = send_colab_tty_resize(&mut ws_write, rows, cols).await;
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn colab_tty_ws_url(proxy_url: &str, proxy_token: &str) -> Result<String> {
    let url = reqwest::Url::parse(proxy_url)
        .map_err(|e| ColabError::config(format!("invalid runtime endpoint: {e}")))?;
    let scheme = if url.scheme() == "http" { "ws" } else { "wss" };
    let host = url
        .host_str()
        .ok_or_else(|| ColabError::config("runtime endpoint has no host"))?;
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    Ok(format!(
        "{scheme}://{host}{port}/colab/tty?colab-runtime-proxy-token={}",
        urlencoding::encode(proxy_token)
    ))
}

async fn send_colab_tty_input<S>(ws_write: &mut S, data: &str) -> Result<()>
where
    S: futures_util::Sink<tungstenite::Message, Error = tungstenite::Error> + Unpin,
{
    ws_write
        .send(tungstenite::Message::Text(
            serde_json::json!({ "data": data }).to_string().into(),
        ))
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket send: {e}")))
}

async fn send_colab_tty_resize<S>(ws_write: &mut S, rows: u16, cols: u16) -> Result<()>
where
    S: futures_util::Sink<tungstenite::Message, Error = tungstenite::Error> + Unpin,
{
    ws_write
        .send(tungstenite::Message::Text(
            serde_json::json!({ "rows": rows, "cols": cols })
                .to_string()
                .into(),
        ))
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket resize: {e}")))
}

fn shell_unavailable_error(error: tungstenite::Error) -> ColabError {
    crate::cocli::debug::debug1("run.shell transport=colab_tty failed");
    crate::cocli::debug::debug3(format!("run.shell websocket error={error}"));
    ColabError::config(
        "Shell is not available on this runtime\nfix: use `colab-cli run py --code \"...\"`\n     or open the browser session with `colab-cli session url --open`",
    )
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub async fn capture_remote_command(
    client: &ColabClient,
    server: &StoredServer,
    command: &str,
) -> Result<String> {
    let term = client
        .create_terminal(&server.proxy_url, &server.proxy_token)
        .await?;

    let ws_url = client.terminal_ws_url(&server.proxy_url, &term.name);
    let request = build_ws_request(&ws_url, &server.proxy_token)?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket connect failed: {e}")))?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let start_marker = format!("__colab_start_{}__", uuid::Uuid::new_v4().simple());
    let end_marker = format!("__colab_end_{}__", uuid::Uuid::new_v4().simple());
    let full_cmd = format!("printf '{start_marker}\\n'; {command}; printf '\\n{end_marker}\\n'\n");
    ws_write
        .send(tungstenite::Message::Text(
            serde_json::json!(["stdin", full_cmd]).to_string().into(),
        ))
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket send: {e}")))?;

    let mut buf = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, ws_read.next()).await {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                if let Some(data) = parse_stdout_frame(text.as_ref()) {
                    buf.push_str(&data);
                    if buf.contains(&end_marker) {
                        break;
                    }
                }
            }
            Ok(Some(Ok(tungstenite::Message::Close(_)))) | Ok(None) => break,
            Err(_) => break,
            _ => continue,
        }
    }

    let start = buf
        .find(&start_marker)
        .map(|i| i + start_marker.len())
        .unwrap_or(0);
    let end = buf.find(&end_marker).unwrap_or(buf.len());
    Ok(buf[start..end].trim().to_string())
}

// long-lived remote process. each ws frame goes to on_chunk; returns when
// cancel fires, the remote closes, or on_chunk returns false.
pub async fn stream_remote_output<F>(
    client: &ColabClient,
    server: &StoredServer,
    command: &str,
    mut on_chunk: F,
    cancel: impl std::future::Future<Output = ()>,
) -> Result<()>
where
    F: FnMut(&str) -> bool,
{
    let term = client
        .create_terminal(&server.proxy_url, &server.proxy_token)
        .await?;

    let ws_url = client.terminal_ws_url(&server.proxy_url, &term.name);
    let request = build_ws_request(&ws_url, &server.proxy_token)?;

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket connect failed: {e}")))?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let msg = serde_json::json!(["stdin", format!("{command}\n")]).to_string();
    ws_write
        .send(tungstenite::Message::Text(msg.into()))
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket send: {e}")))?;

    tokio::pin!(cancel);

    loop {
        tokio::select! {
            _ = &mut cancel => {
                let interrupt = serde_json::json!(["stdin", "\x03"]).to_string();
                let _ = ws_write.send(tungstenite::Message::Text(interrupt.into())).await;
                return Ok(());
            }
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(tungstenite::Message::Text(text))) => {
                        if let Some(data) = parse_stdout_frame(text.as_ref())
                            && !on_chunk(&data)
                        {
                            return Ok(());
                        }
                    }
                    Some(Ok(tungstenite::Message::Close(_))) | None => return Ok(()),
                    Some(Err(e)) => return Err(ColabError::oauth(format!("ws: {e}"))),
                    _ => {}
                }
            }
        }
    }
}

// full-screen remote TUI (bpytop/btop/htop) in alt screen + raw mode.
// reconnects up to 3 times on a transient ws drop, then gives up.
pub async fn run_remote_tui(
    client: &ColabClient,
    server: &StoredServer,
    command: &str,
) -> Result<()> {
    use crossterm::{cursor, execute, terminal as ct_term};

    let term = client
        .create_terminal(&server.proxy_url, &server.proxy_token)
        .await?;
    let terminal_name = term.name.clone();

    // drop guard so we always reap the remote terminal, even on early return
    let cleanup_client = client.clone();
    let cleanup_proxy_url = server.proxy_url.clone();
    let cleanup_proxy_token = server.proxy_token.clone();
    let cleanup_name = terminal_name.clone();
    let _cleanup_guard = TerminalCleanupGuard::new(move || {
        // fire-and-forget on whatever runtime owns this Drop
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = cleanup_client
                    .delete_terminal(&cleanup_proxy_url, &cleanup_proxy_token, &cleanup_name)
                    .await;
            });
        }
    });

    // alt screen + raw mode BEFORE first ws connect, otherwise we flicker
    {
        let mut out = io::stdout();
        execute!(out, ct_term::EnterAlternateScreen, cursor::Hide)
            .map_err(|e| ColabError::config(format!("alt screen: {e}")))?;
    }
    struct AltScreenGuard;
    impl Drop for AltScreenGuard {
        fn drop(&mut self) {
            let mut out = io::stdout();
            let _ = execute!(out, cursor::Show, crossterm::terminal::LeaveAlternateScreen);
            let _ = out.flush();
        }
    }
    let _alt_guard = AltScreenGuard;

    terminal::enable_raw_mode().map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
    let _raw_guard = RawModeGuard;

    // shared channel: stdin reader + resize watcher → async ws writer
    #[derive(Debug)]
    enum WsOut {
        Stdin(Vec<u8>),
        Resize(u16, u16),
    }
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<WsOut>(128);

    // raw stdin reader on a blocking thread — keystroke latency lives here
    let stdin_tx = out_tx.clone();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buf = [0u8; 4096];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if stdin_tx
                        .blocking_send(WsOut::Stdin(buf[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // poll terminal size every 250ms; can't use event::read — clashes with
    // the raw stdin thread on the same fd
    let resize_tx = out_tx.clone();
    let resize_handle = tokio::spawn(async move {
        let mut last = terminal::size().unwrap_or((80, 24));
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(250));
        tick.tick().await;
        loop {
            tick.tick().await;
            let cur = terminal::size().unwrap_or(last);
            if cur != last && resize_tx.send(WsOut::Resize(cur.1, cur.0)).await.is_err() {
                return;
            }
            last = cur;
        }
    });
    let _resize_guard = AbortOnDrop(resize_handle);

    // reconnect loop. clean close → Ok. drop → reattach (3 retries).
    let mut initial_command: Option<String> = Some(command.to_string());
    let mut retries_left: u32 = 3;

    loop {
        let ws_url = client.terminal_ws_url(&server.proxy_url, &terminal_name);
        let request = build_ws_request(&ws_url, &server.proxy_token)?;

        let connect_result = tokio_tungstenite::connect_async(request).await;
        let ws_stream = match connect_result {
            Ok((s, _)) => s,
            Err(_) if retries_left > 0 => {
                retries_left -= 1;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
            Err(e) => {
                return Err(ColabError::oauth(format!("WebSocket connect failed: {e}")));
            }
        };
        let (mut ws_write, mut ws_read) = ws_stream.split();

        // send size first so bpytop doesn't start at 80x24 and redraw
        if let Ok((cols, rows)) = terminal::size() {
            let size_msg = serde_json::json!(["set_size", rows, cols]).to_string();
            let _ = ws_write
                .send(tungstenite::Message::Text(size_msg.into()))
                .await;
        }

        // only send the command on first connect; reattach just watches
        if let Some(cmd) = initial_command.take() {
            let msg = serde_json::json!(["stdin", format!("{cmd}\n")]).to_string();
            let _ = ws_write.send(tungstenite::Message::Text(msg.into())).await;
        }

        let inner = async {
            loop {
                tokio::select! {
                    msg = ws_read.next() => {
                        match msg {
                            Some(Ok(tungstenite::Message::Text(text))) => {
                                if let Some(data) = parse_stdout_frame(text.as_ref()) {
                                    let mut stdout = io::stdout().lock();
                                    let _ = stdout.write_all(data.as_bytes());
                                    let _ = stdout.flush();
                                }
                            }
                            Some(Ok(tungstenite::Message::Binary(bin))) => {
                                let mut stdout = io::stdout().lock();
                                let _ = stdout.write_all(&bin);
                                let _ = stdout.flush();
                            }
                            Some(Ok(tungstenite::Message::Close(_))) | None => {
                                return InnerExit::Closed;
                            }
                            Some(Err(_)) => return InnerExit::Dropped,
                            _ => {}
                        }
                    }
                    out = out_rx.recv() => {
                        let Some(msg) = out else {
                            return InnerExit::Closed;
                        };
                        let serialized = match msg {
                            WsOut::Stdin(bytes) => {
                                let text = String::from_utf8_lossy(&bytes).into_owned();
                                serde_json::json!(["stdin", text]).to_string()
                            }
                            WsOut::Resize(rows, cols) => {
                                serde_json::json!(["set_size", rows, cols]).to_string()
                            }
                        };
                        if ws_write
                            .send(tungstenite::Message::Text(serialized.into()))
                            .await
                            .is_err()
                        {
                            return InnerExit::Dropped;
                        }
                    }
                }
            }
        };

        match inner.await {
            InnerExit::Closed => return Ok(()),
            InnerExit::Dropped if retries_left > 0 => {
                retries_left -= 1;
                // tiny reconnect banner; bpytop's next frame wipes it
                {
                    let mut out = io::stdout();
                    let _ = execute!(
                        out,
                        cursor::MoveTo(0, 0),
                        crossterm::style::Print("  reconnecting\u{2026}  "),
                    );
                    let _ = out.flush();
                }
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                continue;
            }
            InnerExit::Dropped => {
                return Err(ColabError::oauth(
                    "WebSocket dropped and could not reattach",
                ));
            }
        }
    }
}

enum InnerExit {
    Closed,
    Dropped,
}

struct TerminalCleanupGuard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> TerminalCleanupGuard<F> {
    fn new(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
        }
    }
}

impl<F: FnOnce()> Drop for TerminalCleanupGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.cleanup.take() {
            f();
        }
    }
}

fn build_ws_request(ws_url: &str, proxy_token: &str) -> Result<tungstenite::http::Request<()>> {
    tungstenite::http::Request::builder()
        .uri(ws_url)
        .header("X-Colab-Runtime-Proxy-Token", proxy_token)
        .header("X-Colab-Client-Agent", "vscode")
        .header("Host", host_from_url(ws_url))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| ColabError::oauth(format!("failed to build WS request: {e}")))
}

fn kernel_ws_url(proxy_url: &str, kernel_id: &str, session_id: &str) -> String {
    let base = proxy_url
        .trim_end_matches('/')
        .replace("https://", "wss://")
        .replace("http://", "ws://");
    format!("{base}/api/kernels/{kernel_id}/channels?session_id={session_id}")
}

fn collect_cell_message(value: &serde_json::Value, out: &mut CellOutput) {
    match value.pointer("/header/msg_type").and_then(|v| v.as_str()) {
        Some("stream") => {
            let text = value
                .pointer("/content/text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match value.pointer("/content/name").and_then(|v| v.as_str()) {
                Some("stderr") => out.stderr.push_str(text),
                _ => out.stdout.push_str(text),
            }
        }
        Some("execute_result") | Some("display_data") => {
            append_text_plain(value.pointer("/content/data/text/plain"), &mut out.stdout);
        }
        Some("error") => {
            out.error_name = value
                .pointer("/content/ename")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            out.error_value = value
                .pointer("/content/evalue")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if let Some(lines) = value
                .pointer("/content/traceback")
                .and_then(|v| v.as_array())
            {
                out.traceback = lines
                    .iter()
                    .filter_map(|line| line.as_str().map(str::to_string))
                    .collect();
            }
        }
        Some("execute_reply") => {
            if value.pointer("/content/status").and_then(|v| v.as_str()) == Some("error") {
                if out.error_name.is_none() {
                    out.error_name = value
                        .pointer("/content/ename")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                }
                if out.error_value.is_none() {
                    out.error_value = value
                        .pointer("/content/evalue")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                }
            }
        }
        Some("input_request") => {
            out.stderr
                .push_str("kernel requested input; open the Colab session in a browser\n");
        }
        _ => {}
    }
}

fn append_text_plain(value: Option<&serde_json::Value>, out: &mut String) {
    match value {
        Some(serde_json::Value::String(s)) => {
            out.push_str(s);
            out.push('\n');
        }
        Some(serde_json::Value::Array(lines)) => {
            for line in lines {
                if let Some(s) = line.as_str() {
                    out.push_str(s);
                    out.push('\n');
                }
            }
        }
        _ => {}
    }
}

fn host_from_url(url: &str) -> String {
    url.replace("wss://", "")
        .replace("ws://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

// run argv on the remote, stream stdout/stderr through, return its exit code.
// uses printf-marker tricks to skip shell prompt + command echo without
// turning echo off; see run_passthrough_inner.
pub async fn run_passthrough(
    client: &ColabClient,
    server: &StoredServer,
    argv: &[String],
) -> Result<i32> {
    let term = client
        .create_terminal(&server.proxy_url, &server.proxy_token)
        .await?;
    let terminal_name = term.name.clone();

    let result = run_passthrough_inner(client, server, &terminal_name, argv).await;

    // always reap the remote terminal, even on error
    let _ = client
        .delete_terminal(&server.proxy_url, &server.proxy_token, &terminal_name)
        .await;

    result
}

async fn run_passthrough_inner(
    client: &ColabClient,
    server: &StoredServer,
    terminal_name: &str,
    argv: &[String],
) -> Result<i32> {
    let ws_url = client.terminal_ws_url(&server.proxy_url, terminal_name);
    let request = build_ws_request(&ws_url, &server.proxy_token)?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket connect failed: {e}")))?;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    if let Ok((cols, rows)) = terminal::size() {
        let size_msg = serde_json::json!(["set_size", rows, cols]).to_string();
        let _ = ws_write
            .send(tungstenite::Message::Text(size_msg.into()))
            .await;
    }

    let id = uuid::Uuid::new_v4().simple().to_string();
    // marker = 0x01 0x02 colab_<phase>_<uuid> 0x03 0x04. unlikely to collide
    // with user output, and the literal `\001\002...` chars in the wrapper
    // command (what shows up in the PTY echo) decode to different bytes.
    let start_marker: Vec<u8> = {
        let mut v = vec![0x01u8, 0x02];
        v.extend_from_slice(format!("colab_start_{id}").as_bytes());
        v.extend_from_slice(&[0x03, 0x04]);
        v
    };
    let end_marker: Vec<u8> = {
        let mut v = vec![0x01u8, 0x02];
        v.extend_from_slice(format!("colab_end_{id}").as_bytes());
        v.extend_from_slice(&[0x03, 0x04]);
        v
    };

    let user_cmd = argv
        .iter()
        .map(|s| shell_quote(s))
        .collect::<Vec<_>>()
        .join(" ");

    // braces isolate $? for the user command. stderr→stdout because the
    // jupyter terminal only gives us one fd back.
    let wrapped = format!(
        "printf '\\001\\002colab_start_{id}\\003\\004\\n'; \
         {{ {user_cmd}; }} 2>&1; __colab_ec=$?; \
         printf '\\001\\002colab_end_{id}\\003\\004%d\\n' \"$__colab_ec\"\n"
    );

    ws_write
        .send(tungstenite::Message::Text(
            serde_json::json!(["stdin", wrapped]).to_string().into(),
        ))
        .await
        .map_err(|e| ColabError::oauth(format!("WebSocket send: {e}")))?;

    enum Phase {
        Pre,
        Mid,
        Done,
    }
    let mut phase = Phase::Pre;
    let mut buf: Vec<u8> = Vec::new();
    let mut tail_after_end: Vec<u8> = Vec::new();
    let mut exit_code: i32 = 0;

    'outer: loop {
        let msg = match ws_read.next().await {
            Some(m) => m,
            None => break,
        };
        let text = match msg {
            Ok(tungstenite::Message::Text(t)) => t,
            Ok(tungstenite::Message::Close(_)) => break,
            Err(_) => break,
            _ => continue,
        };
        let Some(chunk) = parse_stdout_frame(text.as_ref()) else {
            continue;
        };
        let chunk_bytes = chunk.as_bytes();

        // after END, everything is exit-code digits — skip the scanner
        if matches!(phase, Phase::Done) {
            tail_after_end.extend_from_slice(chunk_bytes);
            if parse_exit_code(&tail_after_end).is_some() {
                break;
            }
            continue;
        }

        buf.extend_from_slice(chunk_bytes);

        loop {
            match phase {
                Phase::Pre => {
                    if let Some(idx) = find_subseq(&buf, &start_marker) {
                        let after = idx + start_marker.len();
                        let after = skip_one_newline(&buf, after);
                        buf.drain(..after);
                        phase = Phase::Mid;
                        continue;
                    }
                    // hold onto marker_len-1 bytes in case it straddles chunks
                    let keep = start_marker.len().saturating_sub(1);
                    if buf.len() > keep {
                        buf.drain(..buf.len() - keep);
                    }
                    break;
                }
                Phase::Mid => {
                    if let Some(idx) = find_subseq(&buf, &end_marker) {
                        if idx > 0 {
                            let mut stdout = io::stdout().lock();
                            let _ = stdout.write_all(&buf[..idx]);
                            let _ = stdout.flush();
                        }
                        let after = idx + end_marker.len();
                        tail_after_end.extend_from_slice(&buf[after..]);
                        buf.clear();
                        phase = Phase::Done;
                        continue;
                    }
                    // flush all but the last marker_len-1 bytes (might be a partial END)
                    let keep = end_marker.len().saturating_sub(1);
                    if buf.len() > keep {
                        let flush_to = buf.len() - keep;
                        let mut stdout = io::stdout().lock();
                        let _ = stdout.write_all(&buf[..flush_to]);
                        let _ = stdout.flush();
                        buf.drain(..flush_to);
                    }
                    break;
                }
                Phase::Done => break,
            }
        }

        if matches!(phase, Phase::Done) && parse_exit_code(&tail_after_end).is_some() {
            break 'outer;
        }
    }

    // ws closed mid-stream — flush whatever's left so we don't drop output
    if matches!(phase, Phase::Mid) && !buf.is_empty() {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(&buf);
        let _ = stdout.flush();
    }

    if let Some(code) = parse_exit_code(&tail_after_end) {
        exit_code = code;
    }

    Ok(exit_code)
}

// byte-for-byte substring search
fn find_subseq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

// eat one \n or \r\n after the START marker so it doesn't show up in output
fn skip_one_newline(buf: &[u8], idx: usize) -> usize {
    if idx >= buf.len() {
        return idx;
    }
    if buf[idx] == b'\r' {
        if idx + 1 < buf.len() && buf[idx + 1] == b'\n' {
            return idx + 2;
        }
        return idx + 1;
    }
    if buf[idx] == b'\n' {
        return idx + 1;
    }
    idx
}

// parse the trailing exit code; needs digit + terminator before returning
fn parse_exit_code(buf: &[u8]) -> Option<i32> {
    let mut s = String::new();
    let mut started = false;
    for &b in buf {
        if b.is_ascii_digit() {
            s.push(b as char);
            started = true;
        } else if started {
            return s.parse::<i32>().ok();
        } else if b == b'\r' || b == b'\n' || b == b' ' {
            continue;
        } else {
            return None;
        }
    }
    None
}

// POSIX single-quote for safe embedding in `sh -c`
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

fn parse_stdout_frame(text: &str) -> Option<String> {
    let arr: Vec<serde_json::Value> = serde_json::from_str(text).ok()?;
    if arr.len() >= 2 && arr[0].as_str() == Some("stdout") {
        arr[1].as_str().map(|s| s.to_string())
    } else {
        None
    }
}

fn parse_colab_tty_frame(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value
        .get("data")
        .and_then(|data| data.as_str())
        .map(str::to_string)
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_from_url_strips_scheme_and_path() {
        assert_eq!(
            host_from_url("wss://abc.proxy.googleusercontent.com/terminals/websocket/1"),
            "abc.proxy.googleusercontent.com"
        );
        assert_eq!(host_from_url("ws://localhost:9000/foo"), "localhost:9000");
    }

    #[test]
    fn host_from_url_no_path() {
        assert_eq!(host_from_url("wss://example.com"), "example.com");
    }

    #[test]
    fn colab_tty_url_uses_root_tty_endpoint() {
        let url = colab_tty_ws_url(
            "https://colab.research.google.com/tun/m/runtime-abc",
            "token value",
        )
        .unwrap();
        assert_eq!(
            url,
            "wss://colab.research.google.com/colab/tty?colab-runtime-proxy-token=token%20value"
        );
        assert!(!url.contains("/api/terminals"));
    }

    #[test]
    fn parses_colab_tty_data_frame() {
        assert_eq!(
            parse_colab_tty_frame(r#"{"data":"HELLO\n"}"#).as_deref(),
            Some("HELLO\n")
        );
        assert_eq!(parse_colab_tty_frame(r#"["stdout","old"]"#), None);
    }

    #[test]
    fn shell_quote_plain() {
        assert_eq!(shell_quote("/content/drive"), "'/content/drive'");
    }

    #[test]
    fn shell_quote_with_embedded_single_quote() {
        assert_eq!(shell_quote("it's/here"), "'it'\"'\"'s/here'");
    }

    #[test]
    fn shell_quote_empty() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn find_subseq_basic() {
        assert_eq!(find_subseq(b"hello world", b"world"), Some(6));
        assert_eq!(find_subseq(b"hello world", b"xyz"), None);
        assert_eq!(find_subseq(b"", b"x"), None);
        assert_eq!(find_subseq(b"abc", b""), None);
    }

    #[test]
    fn skip_one_newline_handles_lf_and_crlf() {
        assert_eq!(skip_one_newline(b"\nrest", 0), 1);
        assert_eq!(skip_one_newline(b"\r\nrest", 0), 2);
        assert_eq!(skip_one_newline(b"\rrest", 0), 1);
        assert_eq!(skip_one_newline(b"rest", 0), 0);
    }

    #[test]
    fn parse_exit_code_simple() {
        assert_eq!(parse_exit_code(b"0\n"), Some(0));
        assert_eq!(parse_exit_code(b"1\n"), Some(1));
        assert_eq!(parse_exit_code(b"127\n"), Some(127));
    }

    #[test]
    fn parse_exit_code_with_whitespace_prefix() {
        assert_eq!(parse_exit_code(b"\r\n42\n"), Some(42));
        assert_eq!(parse_exit_code(b"  3 "), Some(3));
    }

    #[test]
    fn parse_exit_code_incomplete_returns_none() {
        // Digits with no terminator yet — the streamer needs more bytes.
        assert_eq!(parse_exit_code(b"12"), None);
        assert_eq!(parse_exit_code(b""), None);
        assert_eq!(parse_exit_code(b"\r\n"), None);
    }

    #[test]
    fn parse_exit_code_garbage() {
        assert_eq!(parse_exit_code(b"abc"), None);
    }
}
