use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ColabError, Result};

const APP_NAME: &str = "colab-cli";

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
            .or(file.client_id)
            .unwrap_or_default();

        if client_id.is_empty() {
            return Err(ColabError::config(
                "COLAB_EXTENSION_CLIENT_ID is not set \u{2014} add it to your .env file or ~/.config/colab-cli/config.toml",
            ));
        }

        let client_secret = std::env::var("COLAB_EXTENSION_CLIENT_NOT_SO_SECRET")
            .ok()
            .or(file.client_secret)
            .unwrap_or_default();

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

fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| ColabError::config("could not determine config directory"))?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| ColabError::config("could not determine data directory"))?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
