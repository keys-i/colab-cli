use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cocli::error::{ColabError, Result};
pub use crate::cocli::util::paths::{config_dir, config_path, data_dir, write_private};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ColorChoice {
    Always,
    #[default]
    Auto,
    Never,
}

impl std::str::FromStr for ColorChoice {
    type Err = ColabError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "always" => Ok(Self::Always),
            "auto" => Ok(Self::Auto),
            "never" => Ok(Self::Never),
            other => Err(ColabError::config(format!(
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
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_true")]
    pub interactive: bool,
    #[serde(default = "default_true")]
    pub animations: bool,
    #[serde(default)]
    pub bell: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "default_true")]
    pub unicode: bool,
    #[serde(default)]
    pub fun: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: ColorChoice::Auto,
            theme: default_theme(),
            interactive: true,
            animations: true,
            bell: false,
            compact: false,
            unicode: true,
            fun: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(default)]
    pub json: bool,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub timestamps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportConfig {
    #[serde(default = "default_true")]
    pub redact_paths: bool,
    #[serde(default = "default_true")]
    pub redact_emails: bool,
    #[serde(default = "default_true")]
    pub redact_tokens: bool,
}

impl Default for SupportConfig {
    fn default() -> Self {
        Self {
            redact_paths: true,
            redact_emails: true,
            redact_tokens: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CocliConfig {
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub support: SupportConfig,
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

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "auto".to_string()
}

pub fn terminal_bell_allowed(enabled: bool, ci: bool, quiet: bool) -> bool {
    enabled && !ci && !quiet
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColabEnvironment {
    Production,
    Sandbox,
    Local,
}

impl ColabEnvironment {
    fn colab_domain(&self) -> &'static str {
        match self {
            ColabEnvironment::Production => "https://colab.research.google.com",
            ColabEnvironment::Sandbox => "https://colab.sandbox.google.com",
            ColabEnvironment::Local => "https://localhost:8888",
        }
    }
}

impl std::str::FromStr for ColabEnvironment {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "production" => Ok(ColabEnvironment::Production),
            "sandbox" => Ok(ColabEnvironment::Sandbox),
            "local" => Ok(ColabEnvironment::Local),
            other => Err(format!(
                "unknown environment '{other}' \u{2014} expected production, sandbox, or local"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColabConfig {
    pub colab_domain: String,
    pub client_id: String,
    pub client_secret: String,
    pub environment: ColabEnvironment,
    pub data_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct ConfigFile {
    environment: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    colab_domain: Option<String>,
    colab_gapi_domain: Option<String>,
}

impl ColabConfig {
    pub fn load(_quiet: bool) -> Result<Self> {
        let config_dir = config_dir()?;
        let data_dir = data_dir()?;
        let file = load_config_file(&config_dir);

        let env_str = std::env::var("COLAB_EXTENSION_ENVIRONMENT")
            .ok()
            .or(file.environment)
            .unwrap_or_else(|| "production".to_string());

        let environment: ColabEnvironment =
            env_str.parse().map_err(|e: String| ColabError::config(e))?;

        let colab_domain = std::env::var("COLAB_DOMAIN")
            .ok()
            .or(file.colab_domain)
            .unwrap_or_else(|| environment.colab_domain().to_string());

        let client_id = std::env::var("COLAB_EXTENSION_CLIENT_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file.client_id.filter(|s| !s.is_empty()))
            .unwrap_or_else(crate::cocli::util::embedded::embedded_client_id);

        if client_id.is_empty() {
            return Err(ColabError::config(
                "COLAB_EXTENSION_CLIENT_ID is not set \u{2014} add it to your .env file or ~/.config/colab-cli/config.toml",
            ));
        }

        let client_secret = std::env::var("COLAB_EXTENSION_CLIENT_NOT_SO_SECRET")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| file.client_secret.filter(|s| !s.is_empty()))
            .unwrap_or_else(crate::cocli::util::embedded::embedded_client_secret);

        if client_secret.is_empty() {
            return Err(ColabError::config(
                "COLAB_EXTENSION_CLIENT_NOT_SO_SECRET is not set \u{2014} add it to your .env file or ~/.config/colab-cli/config.toml",
            ));
        }

        Ok(Self {
            colab_domain,
            client_id,
            client_secret,
            environment,
            data_dir,
        })
    }

    pub fn servers_file(&self) -> PathBuf {
        self.data_dir.join("servers.json")
    }

    pub fn is_local(&self) -> bool {
        self.environment == ColabEnvironment::Local
    }
}

fn load_config_file(config_dir: &Path) -> ConfigFile {
    let path = config_dir.join("config.toml");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return ConfigFile::default();
    };
    toml::from_str(&contents).unwrap_or_default()
}
