use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{ColabError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccessToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialsFile {
    refresh_token: Option<String>,
    access_token: Option<StoredAccessToken>,
    account: Option<AccountInfo>,
}

pub struct TokenStorage;

impl TokenStorage {
    fn credentials_path() -> Result<PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| ColabError::config("could not determine data directory"))?;
        let dir = base.join("colab-cli");
        fs::create_dir_all(&dir)?;
        Ok(dir.join("credentials.json"))
    }

    fn read() -> Result<CredentialsFile> {
        let path = Self::credentials_path()?;
        match fs::read_to_string(&path) {
            Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CredentialsFile::default()),
            Err(e) => Err(ColabError::Io(e)),
        }
    }

    fn write(creds: &CredentialsFile) -> Result<()> {
        let path = Self::credentials_path()?;
        let json = serde_json::to_string_pretty(creds)?;

        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }

        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn store_refresh_token(token: &str) -> Result<()> {
        let mut creds = Self::read()?;
        creds.refresh_token = Some(token.to_string());
        Self::write(&creds)
    }

    pub fn get_refresh_token() -> Result<Option<String>> {
        Ok(Self::read()?.refresh_token)
    }

    pub fn store_access_token(token: &str, expires_at: DateTime<Utc>) -> Result<()> {
        let mut creds = Self::read()?;
        creds.access_token = Some(StoredAccessToken {
            access_token: token.to_string(),
            expires_at,
        });
        Self::write(&creds)
    }

    pub fn get_access_token() -> Result<Option<StoredAccessToken>> {
        Ok(Self::read()?.access_token)
    }

    pub fn store_account(info: &AccountInfo) -> Result<()> {
        let mut creds = Self::read()?;
        creds.account = Some(info.clone());
        Self::write(&creds)
    }

    pub fn get_account() -> Result<Option<AccountInfo>> {
        Ok(Self::read()?.account)
    }

    pub fn clear_all() -> Result<()> {
        Self::write(&CredentialsFile::default())
    }
}
