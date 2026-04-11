use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::auth::storage::{AccountInfo, TokenStorage};
use crate::config::ColabConfig;
use crate::error::{ColabError, Result};

const FLOW_TIMEOUT_SECS: u64 = 120;
const REDIRECT_SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>Signed in</title></head>
<body style="font-family:sans-serif;text-align:center;padding:4em">
<h1>Signed in to colab-cli</h1>
<p>You can close this tab.</p>
</body></html>"#;

pub const REQUIRED_SCOPES: &[&str] = &[
    "profile",
    "email",
    "https://www.googleapis.com/auth/colaboratory",
];

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    #[allow(dead_code)]
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    name: String,
    email: String,
}

pub async fn run_login_flow(config: &ColabConfig) -> Result<AccountInfo> {
    let (code, redirect_uri, code_verifier) = tokio::time::timeout(
        std::time::Duration::from_secs(FLOW_TIMEOUT_SECS),
        wait_for_auth_code(config),
    )
    .await
    .map_err(|_| ColabError::oauth("authentication timed out (2 min)"))?
    .map_err(|e| ColabError::AuthFailed(e.to_string()))?;

    let http = Client::builder()
        .use_rustls_tls()
        .build()
        .map_err(ColabError::Network)?;
    let tokens = exchange_code(&http, config, &code, &redirect_uri, &code_verifier).await?;

    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in.unwrap_or(3600));

    TokenStorage::store_access_token(&tokens.access_token, expires_at)?;

    let refresh_token = tokens
        .refresh_token
        .ok_or_else(|| ColabError::oauth("no refresh token in response"))?;
    TokenStorage::store_refresh_token(&refresh_token)?;

    let account = fetch_user_info(&http, &tokens.access_token).await?;
    TokenStorage::store_account(&account)?;

    Ok(account)
}

pub async fn refresh_access_token(config: &ColabConfig) -> Result<String> {
    let refresh_token = TokenStorage::get_refresh_token()?.ok_or(ColabError::NotAuthenticated)?;

    let http = Client::builder()
        .use_rustls_tls()
        .build()
        .map_err(ColabError::Network)?;

    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("refresh_token", refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];

    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.ok();
        return Err(ColabError::TokenRefreshFailed {
            reason: body.unwrap_or_else(|| format!("HTTP {status}")),
        });
    }

    let tokens: TokenResponse = resp.json().await?;
    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in.unwrap_or(3600));
    TokenStorage::store_access_token(&tokens.access_token, expires_at)?;

    Ok(tokens.access_token)
}

async fn wait_for_auth_code(config: &ColabConfig) -> Result<(String, String, String)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let nonce: String = {
        use std::fmt::Write;
        let bytes: [u8; 16] = rand_bytes();
        let mut s = String::with_capacity(32);
        for b in &bytes {
            let _ = write!(s, "{b:02x}");
        }
        s
    };

    let (code_verifier, code_challenge) = pkce_pair();

    let auth_url = build_auth_url(config, &redirect_uri, &nonce, &code_challenge);

    if let Err(e) = open_browser(&auth_url) {
        eprintln!("Could not open browser automatically: {e}");
        eprintln!("Open this URL manually:\n  {auth_url}");
    }

    let code = accept_one_redirect(&listener, &nonce).await?;

    Ok((code, redirect_uri, code_verifier))
}

async fn accept_one_redirect(listener: &TcpListener, expected_nonce: &str) -> Result<String> {
    let (stream, _) = listener.accept().await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| ColabError::oauth("malformed HTTP request from browser"))?;

    let url = url::Url::parse(&format!("http://localhost{path}"))
        .map_err(|e| ColabError::oauth(format!("invalid redirect URL: {e}")))?;

    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| ColabError::oauth("missing state in redirect"))?;

    let received_nonce = state
        .strip_prefix("nonce=")
        .ok_or_else(|| ColabError::oauth("invalid state format in redirect"))?;

    if received_nonce != expected_nonce {
        return Err(ColabError::oauth("nonce mismatch — possible CSRF"));
    }

    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| ColabError::oauth("missing authorization code in redirect"))?;

    let body = REDIRECT_SUCCESS_HTML.as_bytes();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    writer.write_all(response.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await?;

    Ok(code)
}

async fn exchange_code(
    http: &Client,
    config: &ColabConfig,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse> {
    let params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.ok();
        return Err(ColabError::oauth(format!(
            "token exchange failed (HTTP {status}): {}",
            body.as_deref().unwrap_or("no body")
        )));
    }

    Ok(resp.json().await?)
}

async fn fetch_user_info(http: &Client, access_token: &str) -> Result<AccountInfo> {
    let resp = http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(ColabError::oauth("failed to fetch user info"));
    }

    let info: UserInfoResponse = resp.json().await?;
    Ok(AccountInfo {
        email: info.email,
        name: info.name,
    })
}

fn pkce_pair() -> (String, String) {
    let verifier_bytes = rand_bytes_n(64);
    let verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    (verifier, challenge)
}

fn rand_bytes() -> [u8; 16] {
    let mut b = [0u8; 16];
    getrandom::getrandom(&mut b).expect("getrandom failed");
    b
}

fn rand_bytes_n(n: usize) -> Vec<u8> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).expect("getrandom failed");
    b
}

fn build_auth_url(
    config: &ColabConfig,
    redirect_uri: &str,
    nonce: &str,
    code_challenge: &str,
) -> String {
    let scopes = REQUIRED_SCOPES.join(" ");
    let encoded_redirect = urlencoding::encode(redirect_uri);
    let encoded_scopes = urlencoding::encode(&scopes);
    let encoded_challenge = urlencoding::encode(code_challenge);
    let state = format!("nonce={nonce}");
    let encoded_state = urlencoding::encode(&state);

    format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
?client_id={}\
&redirect_uri={encoded_redirect}\
&response_type=code\
&scope={encoded_scopes}\
&state={encoded_state}\
&code_challenge={encoded_challenge}\
&code_challenge_method=S256\
&access_type=offline\
&prompt=consent",
        config.client_id
    )
}

fn open_browser(url: &str) -> std::result::Result<(), String> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", url])
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
