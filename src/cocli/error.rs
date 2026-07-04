use thiserror::Error;

#[derive(Debug)]
pub struct DriveError {
    pub kind: String,
    pub message: String,
    pub next_action: Option<String>,
    pub stage: Option<String>,
    pub retryable: bool,
    pub fixes: Vec<String>,
    pub raw: Option<String>,
}

impl std::fmt::Display for DriveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

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

    #[error("Colab returned HTTP {status}")]
    ApiError {
        status: u16,
        url: String,
        body: Option<String>,
    },

    #[error("unexpected API response: {0}")]
    ParseError(String),

    #[error("local config error: {0}")]
    Config(String),

    #[error("{0}")]
    Drive(Box<DriveError>),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),

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

    pub fn drive(
        kind: impl Into<String>,
        message: impl Into<String>,
        next_action: Option<&str>,
        raw: Option<String>,
    ) -> Self {
        Self::Drive(Box::new(DriveError {
            kind: kind.into(),
            message: message.into(),
            next_action: next_action.map(str::to_string),
            stage: None,
            retryable: false,
            fixes: next_action.map(|s| vec![s.to_string()]).unwrap_or_default(),
            raw,
        }))
    }

    pub fn drive_stage(
        kind: impl Into<String>,
        message: impl Into<String>,
        stage: impl Into<String>,
        retryable: bool,
        fixes: Vec<String>,
        raw: Option<String>,
    ) -> Self {
        Self::Drive(Box::new(DriveError {
            kind: kind.into(),
            message: message.into(),
            next_action: fixes.first().cloned(),
            stage: Some(stage.into()),
            retryable,
            fixes,
            raw,
        }))
    }

    pub fn oauth(msg: impl Into<String>) -> Self {
        Self::OAuth(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, ColabError>;
