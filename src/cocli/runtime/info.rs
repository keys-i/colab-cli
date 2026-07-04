//! Colab request helpers that are safe to publish.
//!
//! This crate models public command intent. It does not vendor Google
//! `colabtools` internals.

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ColabModelError {
    #[error("unknown accelerator: {0}")]
    UnknownAccelerator(String),
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, ColabModelError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeClass {
    Cpu,
    Gpu,
    Tpu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accelerator {
    pub class: RuntimeClass,
    pub model: Option<String>,
}

impl Accelerator {
    pub fn cpu() -> Self {
        Self {
            class: RuntimeClass::Cpu,
            model: None,
        }
    }

    pub fn gpu(model: impl Into<String>) -> Self {
        Self {
            class: RuntimeClass::Gpu,
            model: Some(model.into()),
        }
    }

    pub fn tpu(model: impl Into<String>) -> Self {
        Self {
            class: RuntimeClass::Tpu,
            model: Some(model.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    pub name: String,
    pub accelerator: Accelerator,
    pub high_ram: bool,
}

impl SessionCreateRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            accelerator: Accelerator::cpu(),
            high_ram: false,
        }
    }
}

pub fn session_url(colab_domain: &str, endpoint: &str) -> Result<String> {
    let mut url = url::Url::parse(colab_domain.trim_end_matches('/'))?;
    url.set_path(&format!("tun/m/{endpoint}/"));
    Ok(url.to_string())
}

pub fn pip_install_command(packages: &[String]) -> Vec<String> {
    let mut cmd = vec![
        "python".to_string(),
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
    ];
    cmd.extend(packages.iter().cloned());
    cmd
}

pub fn backend_info_url(file: &str) -> String {
    format!("https://raw.githubusercontent.com/googlecolab/backend-info/main/{file}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationHint {
    pub old: String,
    pub new: String,
}

pub fn migration_hint(old_command: &[&str]) -> Option<MigrationHint> {
    let first = old_command.first().copied()?;
    let new = match first {
        "new" => "colab session new",
        "sessions" => "colab session list",
        "status" => "colab status session",
        "stop" => "colab session stop",
        "url" => "colab session url",
        "exec" => "colab run py",
        "run" => "colab run script",
        "upload" => "colab fs push",
        "download" => "colab fs pull",
        "ls" => "colab fs ls",
        "rm" => "colab fs rm",
        "drivemount" => "colab fs drive mount",
        "install" => "colab run pip install",
        _ => return None,
    };
    Some(MigrationHint {
        old: format!("colab {}", old_command.join(" ")),
        new: new.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_urls_use_tunnel_path() {
        assert_eq!(
            session_url("https://colab.research.google.com", "abc").unwrap(),
            "https://colab.research.google.com/tun/m/abc/"
        );
    }

    #[test]
    fn migration_hints_cover_old_exec() {
        let hint = migration_hint(&["exec", "-f", "train.py"]).unwrap();
        assert_eq!(hint.old, "colab exec -f train.py");
        assert_eq!(hint.new, "colab run py");
    }

    #[test]
    fn pip_args_are_arrays() {
        assert_eq!(
            pip_install_command(&["torch".into(), "transformers".into()]),
            vec!["python", "-m", "pip", "install", "torch", "transformers"]
        );
    }
}
