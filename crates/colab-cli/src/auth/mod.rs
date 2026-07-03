pub mod oauth;
pub mod storage;

use std::sync::{Mutex, MutexGuard, OnceLock};

use chrono::Utc;

use crate::auth::storage::StoredAccessToken;
use crate::config::ColabConfig;
use crate::error::{ColabError, Result};

pub use storage::{AccountInfo, TokenStorage};

const REFRESH_MARGIN_SECS: i64 = 5 * 60;

// avoid re-reading credentials.json on every API call
static TOKEN_CACHE: OnceLock<Mutex<Option<StoredAccessToken>>> = OnceLock::new();

fn token_cache() -> &'static Mutex<Option<StoredAccessToken>> {
    TOKEN_CACHE.get_or_init(|| Mutex::new(None))
}

fn token_cache_lock() -> Result<MutexGuard<'static, Option<StoredAccessToken>>> {
    token_cache()
        .lock()
        .map_err(|_| ColabError::config("token cache poisoned"))
}

pub async fn get_access_token(config: &ColabConfig) -> Result<String> {
    // cache
    {
        let guard = token_cache_lock()?;
        if let Some(stored) = guard.as_ref() {
            let remaining = stored.expires_at - Utc::now();
            if remaining.num_seconds() > REFRESH_MARGIN_SECS {
                return Ok(stored.access_token.clone());
            }
        }
    }

    // disk (another colab process might have refreshed it)
    if let Some(stored) = TokenStorage::get_access_token()? {
        let remaining = stored.expires_at - Utc::now();
        if remaining.num_seconds() > REFRESH_MARGIN_SECS {
            let token = stored.access_token.clone();
            *token_cache_lock()? = Some(stored);
            return Ok(token);
        }
    }

    // network
    if TokenStorage::get_refresh_token()?.is_none() {
        return Err(ColabError::NotAuthenticated);
    }
    let token = oauth::refresh_access_token(config).await?;
    if let Some(stored) = TokenStorage::get_access_token()? {
        *token_cache_lock()? = Some(stored);
    }
    Ok(token)
}

pub fn invalidate_token_cache() {
    if let Some(c) = TOKEN_CACHE.get()
        && let Ok(mut guard) = c.lock()
    {
        *guard = None;
    }
}

pub async fn login(config: &ColabConfig) -> Result<AccountInfo> {
    let account = oauth::run_login_flow(config).await?;
    // login wrote a new token; flush stale cache
    invalidate_token_cache();
    Ok(account)
}

pub fn logout() -> Result<()> {
    let result = TokenStorage::clear_all();
    invalidate_token_cache();
    result
}

pub fn current_account() -> Result<Option<AccountInfo>> {
    TokenStorage::get_account()
}
