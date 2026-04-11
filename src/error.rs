use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColabError {
    #[error("not authenticated \u{2014} run `colab-cli auth login` first")]
    NotAuthenticated,

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("token refresh failed: {reason}")]
    TokenRefreshFailed { reason: String },

    #[error("server not found: {endpoint}")]
    ServerNotFound { endpoint: String },

    #[error("too many servers assigned \u{2014} remove one first")]
    TooManyAssignments,

    #[error("insufficient quota to assign this server type")]
    InsufficientQuota,

    #[error("account blocked from Colab servers due to suspected abuse")]
    AccountDenylisted,

    #[error("API request failed: {status} {url}{}", body.as_deref().map(|b| format!("\n  body: {b}")).unwrap_or_default())]
    ApiError {
        status: u16,
        url: String,
        body: Option<String>,
    },

    #[error("unexpected API response: {0}")]
    ParseError(String),

    #[error("local config error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("OAuth2 error: {0}")]
    OAuth(String),
}

impl ColabError {
    pub fn api(status: u16, url: impl Into<String>, body: Option<String>) -> Self {
        Self::ApiError {
            status,
            url: url.into(),
            body,
        }
    }

    pub fn parse(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn oauth(msg: impl Into<String>) -> Self {
        Self::OAuth(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, ColabError>;
