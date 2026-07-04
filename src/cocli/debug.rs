use std::sync::atomic::{AtomicU8, Ordering};

static LEVEL: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    Debug1,
    Debug2,
    Debug3,
}

impl Verbosity {
    pub fn from_count(count: u8, quiet: bool) -> Self {
        if quiet {
            return Self::Quiet;
        }
        match count.min(3) {
            0 => Self::Normal,
            1 => Self::Debug1,
            2 => Self::Debug2,
            _ => Self::Debug3,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::Quiet | Self::Normal => 0,
            Self::Debug1 => 1,
            Self::Debug2 => 2,
            Self::Debug3 => 3,
        }
    }
}

pub fn set(level: Verbosity) {
    LEVEL.store(level.as_u8(), Ordering::Relaxed);
}

pub fn enabled(level: u8) -> bool {
    LEVEL.load(Ordering::Relaxed) >= level
}

pub fn debug1(message: impl AsRef<str>) {
    log(1, message.as_ref());
}

pub fn debug2(message: impl AsRef<str>) {
    log(2, message.as_ref());
}

pub fn debug3(message: impl AsRef<str>) {
    log(3, message.as_ref());
}

pub fn log(level: u8, message: &str) {
    if enabled(level) {
        eprintln!("debug{level}: {}", redact(message));
    }
}

pub fn redact(input: &str) -> String {
    let mut out = redact_query_like(input);
    out = redact_bearer(&out);
    out = redact_header(&out, "cookie:");
    out = redact_header(&out, "authorization:");
    out = redact_header(&out, "x-colab-runtime-proxy-token:");
    out = redact_header(&out, "x-goog-colab-token:");
    if let Some(home) = dirs::home_dir().and_then(|p| p.into_os_string().into_string().ok()) {
        out = out.replace(&home, "~");
    }
    out
}

pub fn sanitize_url(input: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(input) else {
        return redact(input);
    };
    let pairs: Vec<String> = url
        .query_pairs()
        .map(|(key, _)| format!("{key}=<redacted>"))
        .collect();
    if pairs.is_empty() {
        return redact(url.as_str());
    }
    url.set_query(None);
    redact(&format!("{}?{}", url.as_str(), pairs.join("&")))
}

pub fn method_path(url: &str) -> String {
    let Ok(url) = reqwest::Url::parse(url) else {
        return "<unknown>".to_string();
    };
    let path = url.path();
    if let Some(stripped) = path.strip_prefix("/tun/m/") {
        let mut parts = stripped.splitn(2, '/');
        let _endpoint = parts.next();
        if let Some(rest) = parts.next() {
            return format!("/{rest}");
        }
    }
    path.to_string()
}

fn redact_bearer(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let Some(pos) = lower.find("bearer ") else {
        return input.to_string();
    };
    let value_start = pos + "bearer ".len();
    let value_end = input[value_start..]
        .find([' ', '\n', '\r', '\t'])
        .map(|offset| value_start + offset)
        .unwrap_or(input.len());
    format!("{}Bearer <redacted>{}", &input[..pos], &input[value_end..])
}

fn redact_header(input: &str, needle: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let Some(pos) = lower.find(needle) else {
        return input.to_string();
    };
    let end = input[pos..]
        .find('\n')
        .map(|offset| pos + offset)
        .unwrap_or(input.len());
    format!("{}{} <redacted>{}", &input[..pos], needle, &input[end..])
}

fn redact_query_like(input: &str) -> String {
    let mut out = input.to_string();
    for key in [
        "authuser",
        "access_token",
        "refresh_token",
        "api_key",
        "key",
        "token",
        "client_secret",
        "proxy_token",
    ] {
        out = redact_key_values(&out, key);
    }
    out
}

fn redact_key_values(input: &str, key: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    let pattern = format!("{key}=");
    while let Some(pos) = rest.to_ascii_lowercase().find(&pattern) {
        out.push_str(&rest[..pos + pattern.len()]);
        rest = &rest[pos + pattern.len()..];
        if let Some(after) = rest.strip_prefix("<redacted>") {
            out.push_str("<redacted>");
            rest = after;
            continue;
        }
        out.push_str("<redacted>");
        let skip = rest
            .find(['&', ' ', '\n', '\r', '\t', ')', ']', '"', '\'', ','])
            .unwrap_or(rest.len());
        rest = &rest[skip..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{method_path, redact, sanitize_url};

    #[test]
    fn redacts_common_secret_shapes() {
        let text = "Authorization: Bearer abc.def.ghi\nCookie: SID=secret\nauthuser=0&access_token=abc&key=xyz";
        let out = redact(text);
        assert!(!out.contains("abc.def.ghi"));
        assert!(!out.contains("SID=secret"));
        assert!(!out.contains("access_token=abc"));
        assert!(out.contains("access_token=<redacted>"));
        assert!(redact("url (https://x.test?a=1&authuser=0)").contains("authuser=<redacted>)"));
    }

    #[test]
    fn sanitizes_url_query_values() {
        let url = sanitize_url("https://example.test/path?authuser=0&access_token=secret");
        assert!(url.contains("authuser=<redacted>"));
        assert!(!url.contains("secret"));
    }

    #[test]
    fn tunnel_urls_log_logical_api_paths() {
        let path = method_path(
            "https://colab.research.google.com/tun/m/runtime-abc/api/sessions?authuser=0",
        );
        assert_eq!(path, "/api/sessions");
    }
}
