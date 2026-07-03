use serde::{Deserialize, Serialize};

use crate::cocli::auth::profiles::AccountKind;
use crate::cocli::error::{ColabError, Result};
use crate::cocli::util::ids::{DeterministicSeed, secure_seed};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlurpConfig {
    pub slurp: SlurpSection,
    pub budget: Budget,
    #[serde(default)]
    pub accounts: Vec<SlurpAccount>,
    pub work: Work,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub files: Files,
    #[serde(default)]
    pub checkpoint: Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlurpSection {
    pub name: String,
    pub mode: String,
    #[serde(default)]
    pub seed: SeedConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub max_runtime_minutes: u32,
    pub max_compute_units: u32,
    #[serde(default = "default_true")]
    pub stop_on_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlurpAccount {
    pub name: String,
    #[serde(default)]
    pub kind: AccountKind,
    #[serde(default = "one")]
    pub max_runtimes: u32,
    #[serde(default)]
    pub accelerator: Option<String>,
    #[serde(default)]
    pub allow_fallback_account: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Work {
    pub kind: String,
    pub entry: String,
    pub input: String,
    #[serde(default)]
    pub shard_by: Option<String>,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub quant: Option<String>,
    #[serde(default)]
    pub max_memory: Option<String>,
    #[serde(default)]
    pub batch_size: Option<String>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            name: None,
            quant: Some("auto".into()),
            max_memory: Some("auto".into()),
            batch_size: Some("probe".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Files {
    #[serde(default)]
    pub push: Vec<String>,
    #[serde(default)]
    pub pull: Vec<String>,
    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,
}

impl Default for Files {
    fn default() -> Self {
        Self {
            push: Vec::new(),
            pull: Vec::new(),
            exclude: default_excludes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(default = "default_checkpoint_dir")]
    pub dir: String,
    #[serde(default)]
    pub every: Option<String>,
    #[serde(default)]
    pub resume: bool,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Self {
            dir: default_checkpoint_dir(),
            every: None,
            resume: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedState {
    Secure(u64),
    Deterministic(DeterministicSeed),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SeedConfig {
    #[default]
    Secure,
    Deterministic(u64),
}

impl<'de> Deserialize<'de> for SeedConfig {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = SeedConfig;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("\"secure\" or an integer seed")
            }

            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                if v == "secure" {
                    Ok(SeedConfig::Secure)
                } else {
                    Err(E::custom("seed string must be \"secure\""))
                }
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(SeedConfig::Deterministic(v))
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                if v < 0 {
                    Err(E::custom("seed must be non-negative"))
                } else {
                    Ok(SeedConfig::Deterministic(v as u64))
                }
            }
        }

        d.deserialize_any(Visitor)
    }
}

impl Serialize for SeedConfig {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Secure => s.serialize_str("secure"),
            Self::Deterministic(v) => s.serialize_u64(*v),
        }
    }
}

impl SeedConfig {
    pub fn resolve(&self) -> Result<SeedState> {
        match self {
            Self::Secure => Ok(SeedState::Secure(secure_seed()?)),
            Self::Deterministic(v) => Ok(SeedState::Deterministic(DeterministicSeed(*v))),
        }
    }
}

impl SlurpConfig {
    pub fn from_toml_str(s: &str) -> Result<Self> {
        reject_unknown_keys(s)?;
        let cfg: Self = toml::from_str(s)?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn sample() -> &'static str {
        r#"[slurp]
name = "llama-batch-run"
mode = "compliant"
seed = "secure"

[budget]
max_runtime_minutes = 180
max_compute_units = 20
stop_on_budget = true

[[accounts]]
name = "personal-pro"
kind = "colab-paid"
max_runtimes = 1
accelerator = "L4"

[[accounts]]
name = "work-enterprise"
kind = "colab-enterprise"
max_runtimes = 2
accelerator = "A100"

[work]
kind = "sharded-inference"
entry = "jobs/run_inference.py"
input = "data/prompts.jsonl"
shard_by = "lines"
output = "out/results.jsonl"

[model]
name = "local-or-hf-model-name"
quant = "auto"
max_memory = "auto"
batch_size = "probe"

[files]
push = ["jobs/", "data/"]
pull = ["out/", "logs/"]
exclude = [".git", "target", "__pycache__", ".ipynb_checkpoints"]

[checkpoint]
dir = ".cocli/checkpoints"
every = "shard"
resume = true
"#
    }

    pub fn explain(&self) -> String {
        format!(
            "Slurp will run '{}' as {} with {} account profile(s), a {} minute runtime budget, checkpoints in {}, and artifacts pulled from {:?}. It will not bypass Colab rules.",
            self.slurp.name,
            self.work.kind,
            self.accounts.len(),
            self.budget.max_runtime_minutes,
            self.checkpoint.dir,
            self.files.pull
        )
    }

    fn validate(&self) -> Result<()> {
        if self.slurp.mode != "compliant" {
            return Err(ColabError::config(
                "Slurp mode must be \"compliant\" for Colab managed runtimes",
            ));
        }
        if self.accounts.is_empty() {
            return Err(ColabError::config("Slurp needs at least one account"));
        }
        if self.work.entry.trim().is_empty() {
            return Err(ColabError::config("Slurp needs an explicit work.entry"));
        }
        if self.files.push.is_empty() {
            return Err(ColabError::config("Slurp needs explicit files.push"));
        }
        if self.budget.max_runtime_minutes == 0 || self.budget.max_compute_units == 0 {
            return Err(ColabError::config("Slurp needs a non-zero budget"));
        }
        Ok(())
    }
}

fn reject_unknown_keys(s: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(s)?;
    let table = value
        .as_table()
        .ok_or_else(|| ColabError::config("Slurp root must be a TOML table"))?;
    let allowed_root = [
        "slurp",
        "budget",
        "accounts",
        "work",
        "model",
        "files",
        "checkpoint",
    ];
    for key in table.keys() {
        if !allowed_root.contains(&key.as_str()) {
            return Err(ColabError::config(format!(
                "unknown Slurp key '{key}'; did you mean one of slurp, budget, accounts, work, model, files, checkpoint?"
            )));
        }
    }
    check_table(table.get("slurp"), "slurp", &["name", "mode", "seed"])?;
    check_table(
        table.get("budget"),
        "budget",
        &["max_runtime_minutes", "max_compute_units", "stop_on_budget"],
    )?;
    check_table(
        table.get("work"),
        "work",
        &["kind", "entry", "input", "shard_by", "output"],
    )?;
    check_table(
        table.get("model"),
        "model",
        &["name", "quant", "max_memory", "batch_size"],
    )?;
    check_table(table.get("files"), "files", &["push", "pull", "exclude"])?;
    check_table(
        table.get("checkpoint"),
        "checkpoint",
        &["dir", "every", "resume"],
    )?;
    if let Some(toml::Value::Array(accounts)) = table.get("accounts") {
        for account in accounts {
            let Some(t) = account.as_table() else {
                continue;
            };
            check_keys(
                t,
                "accounts",
                &[
                    "name",
                    "kind",
                    "max_runtimes",
                    "accelerator",
                    "allow_fallback_account",
                ],
            )?;
        }
    }
    Ok(())
}

fn check_table(value: Option<&toml::Value>, name: &str, allowed: &[&str]) -> Result<()> {
    if let Some(toml::Value::Table(t)) = value {
        check_keys(t, name, allowed)?;
    }
    Ok(())
}

fn check_keys(
    table: &toml::map::Map<String, toml::Value>,
    name: &str,
    allowed: &[&str],
) -> Result<()> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(ColabError::config(format!(
                "unknown Slurp key '{name}.{key}'"
            )));
        }
    }
    Ok(())
}

fn default_true() -> bool {
    true
}

fn one() -> u32 {
    1
}

fn default_checkpoint_dir() -> String {
    ".cocli/checkpoints".into()
}

fn default_excludes() -> Vec<String> {
    [".git", "target", "__pycache__", ".ipynb_checkpoints"]
        .into_iter()
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_slurp() {
        let cfg = SlurpConfig::from_toml_str(SlurpConfig::sample()).unwrap();
        assert_eq!(cfg.slurp.name, "llama-batch-run");
        assert_eq!(cfg.accounts.len(), 2);
    }

    #[test]
    fn unknown_key_errors() {
        let mut sample = SlurpConfig::sample().to_string();
        sample.push_str("\nwat = true\n");
        assert!(
            SlurpConfig::from_toml_str(&sample)
                .unwrap_err()
                .to_string()
                .contains("unknown")
        );
    }

    #[test]
    fn seed_modes_are_clear() {
        let secure = SlurpConfig::from_toml_str(SlurpConfig::sample()).unwrap();
        assert!(matches!(
            secure.slurp.seed.resolve().unwrap(),
            SeedState::Secure(_)
        ));
        let fixed = SlurpConfig::from_toml_str(
            &SlurpConfig::sample().replace("seed = \"secure\"", "seed = 12345"),
        )
        .unwrap();
        assert!(matches!(
            fixed.slurp.seed.resolve().unwrap(),
            SeedState::Deterministic(_)
        ));
    }
}
