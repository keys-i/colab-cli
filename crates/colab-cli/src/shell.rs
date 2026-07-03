use std::future::Future;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::Arc;

use crossterm::terminal;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite;

use crate::client::ColabClient;
use crate::error::{ColabError, Result};
use crate::server::storage::StoredServer;

// async refresher used by long-running shells to rotate the proxy token.
// returns the new StoredServer so reconnect can pick up the rotated value.
pub type TokenRefresher =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<StoredServer>> + Send>> + Send + Sync>;

pub async fn run_shell(
    client: &ColabClient,
    server: &StoredServer,
    initial_command: Option<&str>,
    refresher: Option<TokenRefresher>,
) -> Result<()> {
    let term = client
        .create_terminal(&server.proxy_url, &server.proxy_token)
        .await?;

    let ws_url = client.terminal_ws_url(&server.proxy_url, &term.name);
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

    // PS1 = "<name> /path #". clear wipes the default prompt that flashed first.
    let label_esc = server.label.replace('\'', "'\\''");
    let prompt_cmd = format!("export PS1='\\[\\e[36m\\]{label_esc}\\[\\e[0m\\] \\w # ' && clear\n");
    let _ = ws_write
        .send(tungstenite::Message::Text(
            serde_json::json!(["stdin", prompt_cmd]).to_string().into(),
        ))
        .await;

    if let Some(cmd) = initial_command {
        let msg = serde_json::json!(["stdin", format!("{cmd}\n")]).to_string();
        let _ = ws_write.send(tungstenite::Message::Text(msg.into())).await;
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

    terminal::enable_raw_mode().map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
    let _raw_guard = RawModeGuard;

    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buf = [0u8; 4096];
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if stdin_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

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
                    Some(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        let msg = serde_json::json!(["stdin", text]).to_string();
                        if ws_write
                            .send(tungstenite::Message::Text(msg.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
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
