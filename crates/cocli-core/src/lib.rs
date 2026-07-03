//! Core config, session lookup, and terminal-output helpers.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("local config error: {0}")]
    Config(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ColorChoice {
    Always,
    #[default]
    Auto,
    Never,
}

impl std::str::FromStr for ColorChoice {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "always" => Ok(Self::Always),
            "auto" => Ok(Self::Auto),
            "never" => Ok(Self::Never),
            other => Err(CoreError::Config(format!(
                "unknown color mode '{other}'; expected auto, always, or never"
            ))),
        }
    }
}

impl ColorChoice {
    pub fn enabled(self, no_color: bool, ci: bool, quiet: bool, json: bool) -> bool {
        if no_color || quiet || json {
            return false;
        }
        match self {
            ColorChoice::Always => true,
            ColorChoice::Auto => !ci,
            ColorChoice::Never => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default)]
    pub color: ColorChoice,
    #[serde(default)]
    pub bell: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: ColorChoice::Auto,
            bell: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CocliConfig {
    #[serde(default)]
    pub ui: UiConfig,
}

impl CocliConfig {
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
        write_private(path, body.as_bytes())
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| CoreError::Config("could not determine config directory".into()))?;
    Ok(base.join("colab-cli"))
}

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| CoreError::Config("could not determine data directory".into()))?;
    Ok(base.join("colab-cli"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
}

pub fn find_session<'a>(sessions: &'a [SessionSummary], name: &str) -> Option<&'a SessionSummary> {
    sessions.iter().find(|s| s.name == name || s.id == name)
}

pub fn terminal_bell_allowed(enabled: bool, ci: bool, quiet: bool) -> bool {
    enabled && !ci && !quiet
}

pub fn parse_days(s: &str) -> Result<u64> {
    let Some(days) = s.strip_suffix('d') else {
        return Err(CoreError::Config(
            "duration must use whole days, for example 7d".into(),
        ));
    };
    days.parse::<u64>()
        .map_err(|_| CoreError::Config(format!("invalid day duration: {s}")))
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_respects_no_color_ci_quiet_json() {
        assert!(ColorChoice::Always.enabled(false, true, false, false));
        assert!(!ColorChoice::Always.enabled(true, false, false, false));
        assert!(!ColorChoice::Auto.enabled(false, true, false, false));
        assert!(!ColorChoice::Auto.enabled(false, false, true, false));
        assert!(!ColorChoice::Auto.enabled(false, false, false, true));
    }

    #[test]
    fn session_lookup_matches_name_or_id() {
        let sessions = vec![SessionSummary {
            id: "abc".into(),
            name: "trainer".into(),
        }];
        assert_eq!(find_session(&sessions, "trainer").unwrap().id, "abc");
        assert_eq!(find_session(&sessions, "abc").unwrap().name, "trainer");
        assert!(find_session(&sessions, "missing").is_none());
    }

    #[test]
    fn config_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = CocliConfig {
            ui: UiConfig {
                color: ColorChoice::Never,
                bell: true,
            },
        };
        cfg.save(&path).unwrap();
        assert_eq!(CocliConfig::load(&path).unwrap(), cfg);
    }

    #[test]
    fn days_require_suffix() {
        assert_eq!(parse_days("7d").unwrap(), 7);
        assert!(parse_days("7").is_err());
    }
}
