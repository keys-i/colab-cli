pub mod api;

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use reqwest::{Client, RequestBuilder, Response, header};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::client::api::{
    Assignment, CcuInfo, GetAssignmentResponse, JupyterTerminal, ListAssignmentsResponse,
    ListedAssignment, Outcome, RuntimeProxyInfo, Session, Shape, Variant,
};
use crate::config::ColabConfig;
use crate::error::{ColabError, Result};

const ACCEPT_JSON: &str = "application/json";
const CLIENT_AGENT: &str = "vscode";
const TUNNEL_HEADER: &str = "X-Colab-Tunnel";
const TUNNEL_VALUE: &str = "Google";
const PROXY_TOKEN_HEADER: &str = "X-Colab-Runtime-Proxy-Token";
const XSRF_TOKEN_HEADER: &str = "X-Goog-Colab-Token";
const CLIENT_AGENT_HEADER: &str = "X-Colab-Client-Agent";
const TUN_PREFIX: &str = "/tun/m";
const XSSI_PREFIX: &[u8] = b")]}'\n";

#[doc(hidden)]
#[inline]
pub fn strip_xssi(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= XSSI_PREFIX.len() && &b[..XSSI_PREFIX.len()] == XSSI_PREFIX {
        unsafe { std::str::from_utf8_unchecked(&b[XSSI_PREFIX.len()..]) }
    } else {
        s
    }
}

type TokenFn = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct ColabClient {
    http: Client,
    colab_domain: String,
    get_access_token: TokenFn,
}

impl ColabClient {
    pub fn new<F, Fut>(config: &ColabConfig, get_access_token: F) -> Result<Self>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        let http = {
            let mut b = Client::builder()
                .use_rustls_tls()
                .tcp_nodelay(true)
                .http2_adaptive_window(true)
                .http2_keep_alive_interval(Duration::from_secs(30))
                .http2_keep_alive_while_idle(true)
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(8)
                .timeout(Duration::from_secs(60))
                .connect_timeout(Duration::from_secs(10));
            if config.is_local() {
                b = b.danger_accept_invalid_certs(true);
            }
            b.build().map_err(ColabError::Network)?
        };

        Ok(Self {
            http,
            colab_domain: config.colab_domain.trim_end_matches('/').to_string(),
            get_access_token: Arc::new(move || Box::pin(get_access_token())),
        })
    }

    pub async fn list_assignments(&self) -> Result<Vec<ListedAssignment>> {
        let url = self.colab_url(format!("{TUN_PREFIX}/assignments"));
        let resp = self.colab_request(self.http.get(&url)).await?;
        let parsed: ListAssignmentsResponse = self.parse_json(resp).await?;
        Ok(parsed.assignments)
    }

    pub async fn assign(
        &self,
        notebook_hash: Uuid,
        variant: Variant,
        accelerator: Option<&str>,
        shape: Shape,
    ) -> Result<(Assignment, bool)> {
        let url = self.build_assign_url(notebook_hash, variant, accelerator, shape);

        let get_resp = self.colab_request(self.http.get(&url)).await?;
        let body = get_resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(strip_xssi(&body))?;

        if json.get("endpoint").is_some() {
            let assignment: Assignment = serde_json::from_value(json)?;
            return Ok((assignment, false));
        }

        let get_response: GetAssignmentResponse = serde_json::from_value(json)?;
        let xsrf_token = get_response.xsrf_token;

        let post_resp = self
            .colab_request(
                self.http
                    .post(&url)
                    .header(XSRF_TOKEN_HEADER, &xsrf_token)
                    .header(header::CONTENT_LENGTH, "0"),
            )
            .await?;
        let assignment: Assignment = self.parse_json(post_resp).await?;

        match assignment.outcome {
            Some(Outcome::QuotaDeniedVariants) | Some(Outcome::QuotaExceededUsageTime) => {
                Err(ColabError::InsufficientQuota)
            }
            Some(Outcome::Denylisted) => Err(ColabError::AccountDenylisted),
            _ => Ok((assignment, true)),
        }
    }

    pub async fn unassign(&self, endpoint: &str) -> Result<()> {
        let url = self.colab_url(format!("{TUN_PREFIX}/unassign/{endpoint}"));

        let token_resp = self.colab_request(self.http.get(&url)).await?;
        let token_body: serde_json::Value = self.parse_json(token_resp).await?;
        let token = token_body["token"]
            .as_str()
            .ok_or_else(|| ColabError::parse("missing token in unassign response"))?
            .to_string();

        self.colab_request(
            self.http
                .post(&url)
                .header(XSRF_TOKEN_HEADER, &token)
                .header(header::CONTENT_LENGTH, "0"),
        )
        .await?;
        Ok(())
    }

    pub async fn refresh_connection(&self, endpoint: &str) -> Result<RuntimeProxyInfo> {
        let url = self.colab_url(format!("{TUN_PREFIX}/runtime-proxy-token"));
        let url = format!("{url}&endpoint={endpoint}&port=8080");
        let resp = self
            .colab_request(self.http.get(&url).header(TUNNEL_HEADER, TUNNEL_VALUE))
            .await?;
        self.parse_json(resp).await
    }

    pub async fn list_sessions_via_tunnel(&self, endpoint: &str) -> Result<Vec<Session>> {
        let url = self.colab_url(format!("{TUN_PREFIX}/{endpoint}/api/sessions"));
        let resp = self
            .colab_request(self.http.get(&url).header(TUNNEL_HEADER, TUNNEL_VALUE))
            .await?;
        self.parse_json(resp).await
    }

    pub async fn delete_session(
        &self,
        proxy_url: &str,
        proxy_token: &str,
        session_id: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/sessions/{session_id}",
            proxy_url.trim_end_matches('/')
        );
        let resp = self
            .http
            .delete(&url)
            .header(PROXY_TOKEN_HEADER, proxy_token)
            .header(CLIENT_AGENT_HEADER, CLIENT_AGENT)
            .send()
            .await?;
        self.check_status_raw(resp, &url).await?;
        Ok(())
    }

    pub async fn create_terminal(
        &self,
        proxy_url: &str,
        proxy_token: &str,
    ) -> Result<JupyterTerminal> {
        let url = format!("{}/api/terminals", proxy_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .header(PROXY_TOKEN_HEADER, proxy_token)
            .header(CLIENT_AGENT_HEADER, CLIENT_AGENT)
            .header(header::ACCEPT, ACCEPT_JSON)
            .header(header::CONTENT_LENGTH, "0")
            .send()
            .await?;
        let resp = self.check_status_raw(resp, &url).await?;
        Ok(resp.json().await?)
    }

    /// Delete a Jupyter terminal that was previously created with
    /// `create_terminal`. Used to cleanly reap the remote process tree
    /// belonging to a specific short-lived view (e.g. `server ps`) without
    /// touching unrelated sessions or the assigned server itself.
    pub async fn delete_terminal(
        &self,
        proxy_url: &str,
        proxy_token: &str,
        terminal_name: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/terminals/{}",
            proxy_url.trim_end_matches('/'),
            terminal_name
        );
        let resp = self
            .http
            .delete(&url)
            .header(PROXY_TOKEN_HEADER, proxy_token)
            .header(CLIENT_AGENT_HEADER, CLIENT_AGENT)
            .send()
            .await?;
        // A 404 here means the terminal was already reaped by the remote
        // (e.g. because the user's bpytop exited and the shell walked out
        // of its parent). That's not an error from our perspective.
        if resp.status().as_u16() == 404 {
            return Ok(());
        }
        self.check_status_raw(resp, &url).await?;
        Ok(())
    }

    pub fn terminal_ws_url(&self, proxy_url: &str, terminal_name: &str) -> String {
        let base = proxy_url
            .trim_end_matches('/')
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{base}/terminals/websocket/{terminal_name}")
    }

    pub async fn upload_file_streaming(
        &self,
        proxy_url: &str,
        proxy_token: &str,
        remote_path: &str,
        file_path: &Path,
        progress: impl Fn(u64) + Send + 'static,
    ) -> Result<()> {
        // url-encode each segment so paths with spaces work; keep `/` as-is
        let encoded_path = remote_path
            .trim_start_matches('/')
            .split('/')
            .map(|seg| urlencoding::encode(seg).into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let url = format!(
            "{}/api/contents/{encoded_path}",
            proxy_url.trim_end_matches('/')
        );

        let meta = std::fs::metadata(file_path)?;
        if !meta.is_file() {
            return Err(ColabError::config(format!(
                "not a regular file: {}",
                file_path.display()
            )));
        }
        let file_size = meta.len();

        const CHUNK_RAW: usize = 3 * 1024 * 1024;

        let prefix = br#"{"type":"file","format":"base64","content":""#;
        let suffix = br#""}"#;
        let base64_len = (file_size.div_ceil(3) * 4) as usize;
        let content_length = prefix.len() + base64_len + suffix.len();

        let file_path = file_path.to_owned();
        let (tx, rx) =
            tokio::sync::mpsc::channel::<std::result::Result<Vec<u8>, std::io::Error>>(4);

        tokio::task::spawn_blocking(move || {
            use std::io::Read;

            if tx.blocking_send(Ok(prefix.to_vec())).is_err() {
                return;
            }

            let mut file = match std::fs::File::open(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
            };

            let mut buf = vec![0u8; CHUNK_RAW];
            let mut bytes_so_far = 0u64;

            loop {
                let mut filled = 0;
                while filled < CHUNK_RAW {
                    match file.read(&mut buf[filled..]) {
                        Ok(0) => break,
                        Ok(n) => filled += n,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(e));
                            return;
                        }
                    }
                }
                if filled == 0 {
                    break;
                }
                bytes_so_far += filled as u64;
                progress(bytes_so_far);
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(&buf[..filled])
                    .into_bytes();
                if tx.blocking_send(Ok(encoded)).is_err() {
                    return;
                }
            }

            let _ = tx.blocking_send(Ok(suffix.to_vec()));
        });

        let stream = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        let body = reqwest::Body::wrap_stream(stream);

        let resp = self
            .http
            .put(&url)
            .header(PROXY_TOKEN_HEADER, proxy_token)
            .header(CLIENT_AGENT_HEADER, CLIENT_AGENT)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, content_length.to_string())
            .body(body)
            .send()
            .await?;

        self.check_status_raw(resp, &url).await?;
        Ok(())
    }

    pub async fn send_keep_alive(&self, endpoint: &str) -> Result<()> {
        let url = self.colab_url(format!("{TUN_PREFIX}/{endpoint}/keep-alive/"));
        self.colab_request(
            self.http
                .post(&url)
                .header(TUNNEL_HEADER, TUNNEL_VALUE)
                .header(header::CONTENT_LENGTH, "0"),
        )
        .await?;
        Ok(())
    }

    pub async fn get_ccu_info(&self) -> Result<CcuInfo> {
        let url = self.colab_url(format!("{TUN_PREFIX}/ccu-info"));
        let resp = self.colab_request(self.http.get(&url)).await?;
        self.parse_json(resp).await
    }

    #[inline]
    fn colab_url(&self, path: impl AsRef<str>) -> String {
        let mut out = String::with_capacity(self.colab_domain.len() + path.as_ref().len() + 10);
        out.push_str(&self.colab_domain);
        out.push_str(path.as_ref());
        out.push_str("?authuser=0");
        out
    }

    fn build_assign_url(
        &self,
        notebook_hash: Uuid,
        variant: Variant,
        accelerator: Option<&str>,
        shape: Shape,
    ) -> String {
        build_assign_url(
            &self.colab_domain,
            notebook_hash,
            variant,
            accelerator,
            shape,
        )
    }

    async fn colab_request(&self, builder: RequestBuilder) -> Result<Response> {
        let token = (self.get_access_token)().await?;
        let resp = builder
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::ACCEPT, ACCEPT_JSON)
            .header(CLIENT_AGENT_HEADER, CLIENT_AGENT)
            .send()
            .await?;
        let url = resp.url().to_string();
        self.check_status_raw(resp, &url).await
    }

    async fn check_status_raw(&self, resp: Response, url: &str) -> Result<Response> {
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status().as_u16();
        let body = resp.text().await.ok();
        match status {
            412 => Err(ColabError::TooManyAssignments),
            404 => Err(ColabError::ServerNotFound {
                endpoint: url.to_string(),
            }),
            _ => Err(ColabError::api(status, url, body)),
        }
    }

    async fn parse_json<T: DeserializeOwned>(&self, resp: Response) -> Result<T> {
        let body = resp.text().await?;
        serde_json::from_str(strip_xssi(&body)).map_err(|e| {
            ColabError::parse(format!("failed to parse API response: {e}\nbody: {body}"))
        })
    }
}

#[doc(hidden)]
pub fn build_assign_url(
    colab_domain: &str,
    notebook_hash: Uuid,
    variant: Variant,
    accelerator: Option<&str>,
    shape: Shape,
) -> String {
    let nbh = uuid_to_websafe_base64(notebook_hash);
    let mut url = String::with_capacity(colab_domain.len() + 96);
    url.push_str(colab_domain);
    url.push_str(TUN_PREFIX);
    url.push_str("/assign?authuser=0&nbh=");
    url.push_str(&nbh);
    if !matches!(variant, Variant::Cpu) {
        url.push_str("&variant=");
        url.push_str(variant_param(variant));
    }
    if let Some(acc) = accelerator {
        url.push_str("&accelerator=");
        url.push_str(acc);
    }
    // High-RAM is requested via `&shape=hm` — this matches exactly what the
    // Colab web UI sends (see network capture). There is no `machineShape=N`
    // parameter; Standard omits the param entirely.
    if matches!(shape, Shape::HighMem) {
        url.push_str("&shape=hm");
    }
    url
}

#[doc(hidden)]
#[inline]
pub fn uuid_to_websafe_base64(id: Uuid) -> String {
    let s = id.to_string().replace('-', "_");
    let mut out = String::with_capacity(44);
    out.push_str(&s);
    for _ in s.len()..44 {
        out.push('.');
    }
    out
}

#[inline]
fn variant_param(v: Variant) -> &'static str {
    match v {
        Variant::Cpu => "DEFAULT",
        Variant::Gpu => "GPU",
        Variant::Tpu => "TPU",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_xssi_removes_prefix_when_present() {
        assert_eq!(strip_xssi(")]}'\n{\"a\":1}"), "{\"a\":1}");
    }

    #[test]
    fn strip_xssi_is_identity_without_prefix() {
        assert_eq!(strip_xssi("{\"a\":1}"), "{\"a\":1}");
    }

    #[test]
    fn strip_xssi_handles_empty() {
        assert_eq!(strip_xssi(""), "");
    }

    #[test]
    fn uuid_encodes_to_44_char_websafe() {
        let id = Uuid::nil();
        let nbh = uuid_to_websafe_base64(id);
        assert_eq!(nbh.len(), 44);
        assert!(nbh.starts_with("00000000_0000_0000_0000_000000000000"));
        assert!(nbh.ends_with('.'));
        assert!(!nbh.contains('-'));
    }

    #[test]
    fn uuid_round_trips_a_real_uuid() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let nbh = uuid_to_websafe_base64(id);
        assert_eq!(nbh.len(), 44);
        assert_eq!(&nbh[..36], "550e8400_e29b_41d4_a716_446655440000");
        assert_eq!(&nbh[36..], "........");
    }

    #[test]
    fn variant_param_mapping() {
        assert_eq!(variant_param(Variant::Cpu), "DEFAULT");
        assert_eq!(variant_param(Variant::Gpu), "GPU");
        assert_eq!(variant_param(Variant::Tpu), "TPU");
    }

    #[test]
    fn assign_url_cpu_standard_is_minimal() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let u = build_assign_url(
            "https://colab.research.google.com",
            id,
            Variant::Cpu,
            None,
            Shape::Standard,
        );
        assert!(u.contains("/tun/m/assign?authuser=0"));
        assert!(u.contains("&nbh=550e8400_e29b_41d4_a716_446655440000"));
        assert!(!u.contains("variant="));
        assert!(!u.contains("accelerator="));
        assert!(!u.contains("shape="));
        assert!(!u.contains("machineShape="));
    }

    #[test]
    fn assign_url_gpu_with_accelerator_and_highmem() {
        let id = Uuid::nil();
        let u = build_assign_url(
            "https://colab.research.google.com",
            id,
            Variant::Gpu,
            Some("T4"),
            Shape::HighMem,
        );
        assert!(u.contains("variant=GPU"));
        assert!(u.contains("accelerator=T4"));
        // High-RAM is signalled with `&shape=hm` — matches colab.research.google.com web UI.
        assert!(u.contains("&shape=hm"));
        assert!(!u.contains("machineShape="));
    }

    #[test]
    fn assign_url_tpu_no_accelerator_standard() {
        let id = Uuid::nil();
        let u = build_assign_url("https://x.y", id, Variant::Tpu, None, Shape::Standard);
        assert!(u.contains("variant=TPU"));
        assert!(!u.contains("accelerator="));
        assert!(!u.contains("shape="));
    }
}
