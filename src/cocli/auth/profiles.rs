use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cocli::error::{ColabError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AccountKind {
    ColabFree,
    ColabPaid,
    ColabEnterprise,
    GcpMarketplace,
    Local,
    #[default]
    Unknown,
}

impl AccountKind {
    pub fn allows_fleet(self) -> bool {
        matches!(
            self,
            Self::ColabPaid | Self::ColabEnterprise | Self::GcpMarketplace | Self::Local
        )
    }
}

impl std::str::FromStr for AccountKind {
    type Err = ColabError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "colab-free" => Ok(Self::ColabFree),
            "colab-paid" => Ok(Self::ColabPaid),
            "colab-enterprise" => Ok(Self::ColabEnterprise),
            "gcp-marketplace" => Ok(Self::GcpMarketplace),
            "local" => Ok(Self::Local),
            "unknown" => Ok(Self::Unknown),
            other => Err(ColabError::config(format!(
                "unknown account kind '{other}'; expected colab-paid, colab-enterprise, gcp-marketplace, local, or unknown"
            ))),
        }
    }
}

impl std::fmt::Display for AccountKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ColabFree => "colab-free",
            Self::ColabPaid => "colab-paid",
            Self::ColabEnterprise => "colab-enterprise",
            Self::GcpMarketplace => "gcp-marketplace",
            Self::Local => "local",
            Self::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StorageBackend {
    Keyring,
    #[default]
    Session,
    EncryptedFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthProfile {
    pub name: String,
    #[serde(default)]
    pub account_hint: Option<String>,
    #[serde(default)]
    pub kind: AccountKind,
    pub created_at: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub storage_backend: StorageBackend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthProfiles {
    #[serde(default)]
    pub profiles: Vec<AuthProfile>,
    #[serde(default)]
    pub active: Option<String>,
}

impl AuthProfiles {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)?;
        crate::cocli::util::paths::write_private(path, body.as_bytes())
    }

    pub fn add(&mut self, profile: AuthProfile) -> Result<()> {
        if self.profiles.iter().any(|p| p.name == profile.name) {
            return Err(ColabError::config(format!(
                "auth profile already exists: {}",
                profile.name
            )));
        }
        self.profiles.push(profile);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&AuthProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        self.profiles.len() != before
    }
}

pub fn redacted_email(s: &str, show_private: bool) -> String {
    if show_private {
        return s.to_string();
    }
    let Some((_, domain)) = s.split_once('@') else {
        return "<redacted>".to_string();
    };
    format!("<redacted>@{domain}")
}

pub fn redact_sensitive(input: &str) -> String {
    let mut out = input.to_string();
    for key in [
        "access_token",
        "refresh_token",
        "client_secret",
        "api_key",
        "cookie",
        "authorization",
    ] {
        out = redact_key(&out, key);
    }
    out
}

fn redact_key(input: &str, key: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains(key) {
            out.push_str(key);
            out.push_str(" = \"<redacted>\"\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_serialize_without_tokens() {
        let profiles = AuthProfiles {
            profiles: vec![AuthProfile {
                name: "personal".into(),
                account_hint: Some("me@example.com".into()),
                kind: AccountKind::ColabPaid,
                created_at: "now".into(),
                last_used_at: None,
                storage_backend: StorageBackend::Session,
            }],
            active: Some("personal".into()),
        };
        let toml = toml::to_string(&profiles).unwrap();
        assert!(toml.contains("personal"));
        assert!(!toml.contains("token"));
        assert!(!toml.contains("secret"));
    }

    #[test]
    fn redaction_hides_tokens_and_email_local_part() {
        let text = "access_token = \"abc\"\nrefresh_token = \"def\"\n";
        let redacted = redact_sensitive(text);
        assert!(!redacted.contains("abc"));
        assert!(!redacted.contains("def"));
        assert_eq!(
            redacted_email("person@example.com", false),
            "<redacted>@example.com"
        );
        assert_eq!(
            redacted_email("person@example.com", true),
            "person@example.com"
        );
    }
}
