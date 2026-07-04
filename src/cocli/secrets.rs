use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::cocli::error::{ColabError, Result};

#[derive(Default, Debug, Clone)]
pub struct SecretCliArgs {
    pub env: Vec<String>,
    pub env_file: Vec<String>,
    pub secret: Vec<String>,
}

impl SecretCliArgs {
    pub fn is_empty(&self) -> bool {
        self.env.is_empty() && self.env_file.is_empty() && self.secret.is_empty()
    }
}

#[derive(Default)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Debug)]
pub struct ResolvedSecret {
    pub remote_key: String,
    pub source: String,
    pub value: SecretValue,
}

#[derive(Default, Debug)]
pub struct SecretBundle {
    values: Vec<ResolvedSecret>,
}

impl SecretBundle {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn rows(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|s| (s.remote_key.as_str(), s.source.as_str()))
    }

    pub fn env_pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|s| (s.remote_key.as_str(), s.value.expose()))
    }

    pub fn redact_text(&self, text: &str) -> String {
        let mut redacted = redact_common_token_patterns(text);
        for secret in &self.values {
            let value = secret.value.expose();
            if !value.is_empty() {
                redacted = redacted.replace(value, "<redacted>");
            }
        }
        redacted
    }

    pub fn python_prelude(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let values = self
            .values
            .iter()
            .map(|secret| {
                (
                    secret.remote_key.clone(),
                    serde_json::Value::String(secret.value.expose().to_string()),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let json = serde_json::Value::Object(values).to_string();
        format!(
            r#"
import os as _colab_cli_os
_colab_cli_cli_secrets = {json}
_colab_cli_os.environ.update(_colab_cli_cli_secrets)
try:
    from google.colab import userdata as _colab_cli_userdata
    def _colab_cli_userdata_get(key):
        if not isinstance(key, str) or not key or any(ch.isspace() for ch in key):
            raise ValueError("Secret key must be a non-empty string without whitespace")
        if key in _colab_cli_cli_secrets:
            return _colab_cli_cli_secrets[key]
        raise KeyError("Secret was not provided to the CLI secrets bridge: %s" % key)
    _colab_cli_userdata.get = _colab_cli_userdata_get
except Exception:
    pass
"#
        )
    }
}

pub fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return Err(ColabError::config(
            "secret key must be a non-empty string without whitespace",
        ));
    }
    Ok(())
}

pub fn resolve_from_process_env(args: &SecretCliArgs) -> Result<SecretBundle> {
    resolve(args, |key| std::env::var(key).ok())
}

pub fn resolve(
    args: &SecretCliArgs,
    lookup_env: impl Fn(&str) -> Option<String>,
) -> Result<SecretBundle> {
    let mut values = BTreeMap::<String, ResolvedSecret>::new();
    for spec in &args.env {
        let parsed = parse_env_spec(spec)?;
        let value = match parsed.value {
            Some(value) => value,
            None => lookup_env(&parsed.local_key).ok_or_else(|| {
                ColabError::config(format!(
                    "Missing secret: {}\nfix: export {}=... or run colab secret set {} --prompt",
                    parsed.local_key, parsed.local_key, parsed.remote_key
                ))
            })?,
        };
        values.insert(
            parsed.remote_key.clone(),
            ResolvedSecret {
                remote_key: parsed.remote_key,
                source: parsed.source,
                value: SecretValue::new(value),
            },
        );
    }

    for path in &args.env_file {
        for (key, value) in parse_dotenv_file(Path::new(path))? {
            values.insert(
                key.clone(),
                ResolvedSecret {
                    remote_key: key,
                    source: format!("env-file:{path}"),
                    value: SecretValue::new(value),
                },
            );
        }
    }

    for spec in &args.secret {
        let (remote_key, local_key) = match spec.split_once('=') {
            Some((remote, local)) => (remote.to_string(), local.to_string()),
            None => (spec.clone(), spec.clone()),
        };
        validate_key(&remote_key)?;
        validate_key(&local_key)?;
        let value = lookup_env(&local_key).ok_or_else(|| {
            ColabError::config(format!(
                "Missing secret: {local_key}\nfix: export {local_key}=... or run colab secret set {remote_key} --prompt"
            ))
        })?;
        values.insert(
            remote_key.clone(),
            ResolvedSecret {
                remote_key,
                source: "local env fallback".to_string(),
                value: SecretValue::new(value),
            },
        );
    }

    Ok(SecretBundle {
        values: values.into_values().collect(),
    })
}

struct ParsedEnvSpec {
    remote_key: String,
    local_key: String,
    value: Option<String>,
    source: String,
}

fn parse_env_spec(spec: &str) -> Result<ParsedEnvSpec> {
    if let Some((key, value)) = spec.split_once('=') {
        validate_key(key)?;
        return Ok(ParsedEnvSpec {
            remote_key: key.to_string(),
            local_key: key.to_string(),
            value: Some(value.to_string()),
            source: "cli argument".to_string(),
        });
    }
    if let Some((remote, local)) = spec.split_once(':') {
        validate_key(remote)?;
        validate_key(local)?;
        return Ok(ParsedEnvSpec {
            remote_key: remote.to_string(),
            local_key: local.to_string(),
            value: None,
            source: format!("local env:{local}"),
        });
    }
    validate_key(spec)?;
    Ok(ParsedEnvSpec {
        remote_key: spec.to_string(),
        local_key: spec.to_string(),
        value: None,
        source: "local env".to_string(),
    })
}

pub fn parse_dotenv_file(path: &Path) -> Result<Vec<(String, String)>> {
    let body = std::fs::read_to_string(path)?;
    parse_dotenv(&body)
}

pub fn parse_dotenv(body: &str) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        validate_key(key)?;
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        out.push((key.to_string(), value));
    }
    Ok(out)
}

pub fn userdata_reply(key: &str, bundle: &SecretBundle) -> Result<serde_json::Value> {
    validate_key(key)?;
    for (remote, value) in bundle.env_pairs() {
        if remote == key {
            return Ok(serde_json::json!({
                "exists": true,
                "access": true,
                "payload": value,
            }));
        }
    }
    Ok(serde_json::json!({
        "exists": false,
        "access": false,
        "payload": serde_json::Value::Null,
    }))
}

fn redact_common_token_patterns(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            out.push_str(&redact_token(&token));
            token.clear();
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    out.push_str(&redact_token(&token));
    out
}

fn redact_token(token: &str) -> String {
    let lower = token.to_ascii_lowercase();
    if lower.starts_with("hf_")
        || lower.starts_with("sk-")
        || lower.contains("authorization:")
        || lower.contains("access_token=")
        || lower.contains("api_key=")
    {
        "<redacted>".to_string()
    } else {
        token.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_value_redacts_debug_and_display() {
        let value = SecretValue::new("hf_secret".to_string());
        assert_eq!(format!("{value:?}"), "<redacted>");
        assert_eq!(format!("{value}"), "<redacted>");
    }

    #[test]
    fn rejects_empty_or_whitespace_keys() {
        assert!(validate_key("").is_err());
        assert!(validate_key("BAD KEY").is_err());
        assert!(validate_key("HF_TOKEN").is_ok());
    }

    #[test]
    fn dotenv_parser_handles_comments_and_quotes() {
        let parsed = parse_dotenv(
            r#"
# comment
HF_TOKEN="hf_123"
WANDB_API_KEY='wandb'
"#,
        )
        .unwrap();
        assert_eq!(parsed[0], ("HF_TOKEN".to_string(), "hf_123".to_string()));
        assert_eq!(
            parsed[1],
            ("WANDB_API_KEY".to_string(), "wandb".to_string())
        );
    }

    #[test]
    fn env_mapping_supports_local_and_direct_values() {
        let args = SecretCliArgs {
            env: vec![
                "HF_TOKEN".to_string(),
                "REMOTE:LOCAL".to_string(),
                "DIRECT=value".to_string(),
            ],
            ..Default::default()
        };
        let bundle = resolve(&args, |key| match key {
            "HF_TOKEN" => Some("hf_local".to_string()),
            "LOCAL" => Some("mapped".to_string()),
            _ => None,
        })
        .unwrap();
        let pairs = bundle.env_pairs().collect::<Vec<_>>();
        assert!(pairs.contains(&("HF_TOKEN", "hf_local")));
        assert!(pairs.contains(&("REMOTE", "mapped")));
        assert!(pairs.contains(&("DIRECT", "value")));
    }

    #[test]
    fn missing_env_has_helpful_error() {
        let args = SecretCliArgs {
            env: vec!["HF_TOKEN".to_string()],
            ..Default::default()
        };
        let err = resolve(&args, |_| None).unwrap_err().to_string();
        assert!(err.contains("Missing secret: HF_TOKEN"));
        assert!(err.contains("export HF_TOKEN"));
    }

    #[test]
    fn userdata_reply_shapes_match_colab_expectation() {
        let args = SecretCliArgs {
            env: vec!["HF_TOKEN".to_string()],
            ..Default::default()
        };
        let bundle = resolve(&args, |key| {
            (key == "HF_TOKEN").then(|| "hf_local".to_string())
        })
        .unwrap();
        assert_eq!(userdata_reply("HF_TOKEN", &bundle).unwrap()["exists"], true);
        assert_eq!(userdata_reply("MISSING", &bundle).unwrap()["exists"], false);
    }

    #[test]
    fn redacts_exact_values_and_common_tokens() {
        let args = SecretCliArgs {
            env: vec!["HF_TOKEN".to_string()],
            ..Default::default()
        };
        let bundle = resolve(&args, |_| Some("secret-value".to_string())).unwrap();
        let text = bundle.redact_text("token secret-value hf_abc123");
        assert!(!text.contains("secret-value"));
        assert!(!text.contains("hf_abc123"));
    }
}
