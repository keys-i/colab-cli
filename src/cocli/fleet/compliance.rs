use serde::{Deserialize, Serialize};

use crate::cocli::auth::profiles::AccountKind;
use crate::cocli::slurp::SlurpConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceLevel {
    Ok,
    Warn,
    Refuse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub level: ComplianceLevel,
    pub message: String,
    pub next_action: String,
}

pub fn validate_slurp(cfg: &SlurpConfig) -> Vec<ComplianceFinding> {
    let mut out = Vec::new();
    let total_runtimes: u32 = cfg.accounts.iter().map(|a| a.max_runtimes).sum();
    let unknown_or_free = cfg
        .accounts
        .iter()
        .filter(|a| matches!(a.kind, AccountKind::Unknown | AccountKind::ColabFree))
        .count();

    if total_runtimes > 1 && unknown_or_free > 0 {
        out.push(refuse(
            "blocked: this looks like account rotation to dodge limits",
            "use paid, enterprise, marketplace, or local runtimes for distribute jobs",
        ));
    }

    for account in &cfg.accounts {
        if !account.kind.allows_fleet() && account.max_runtimes > 1 {
            out.push(refuse(
                "too many runtimes requested for a non-paid profile",
                "set max_runtimes = 1 or configure a paid, enterprise, marketplace, or local profile",
            ));
        }
        if account.allow_fallback_account && !account.kind.allows_fleet() {
            out.push(refuse(
                "fallback account rotation is blocked for unknown/free profiles",
                "remove allow_fallback_account or use an approved profile kind",
            ));
        }
    }

    if cfg.work.kind.contains("distributed") && cfg.accounts.iter().any(|a| !a.kind.allows_fleet())
    {
        out.push(refuse(
            "distributed task kind is blocked for free or unknown managed runtimes",
            "use paid, enterprise, marketplace, or local runtimes",
        ));
    }

    let visible = format!(
        "{} {} {} {:?}",
        cfg.work.kind, cfg.work.entry, cfg.work.input, cfg.files.push
    )
    .to_ascii_lowercase();
    for pattern in [
        "keepalive only",
        "anti-idle",
        "crypto mining",
        "xmrig",
        "password cracking",
        "hashcat",
        "proxy server",
        "web ui",
    ] {
        if visible.contains(pattern) {
            out.push(refuse(
                "blocked: workload pattern is not appropriate for managed runtimes",
                "remove that workload from the recipe",
            ));
        }
    }

    if cfg.checkpoint.every.is_none() && cfg.budget.max_runtime_minutes > 60 {
        out.push(warn(
            "long distribute job has no checkpoint cadence",
            "set [checkpoint].every = \"shard\"",
        ));
    }
    if cfg.files.pull.is_empty() {
        out.push(warn(
            "no explicit artifact pull configured",
            "add [files].pull outputs so finished work is collected",
        ));
    }
    if cfg
        .files
        .push
        .iter()
        .any(|p| p.contains("/drive") || p.contains("drive/"))
    {
        out.push(warn(
            "Drive paths can be a bottleneck for shard fan-out",
            "push hot inputs to /content or a local cache first",
        ));
    }

    if out.is_empty() {
        out.push(ComplianceFinding {
            level: ComplianceLevel::Ok,
            message: "recipe can be planned without bypassing Colab rules".into(),
            next_action: "run distribute plan before start".into(),
        });
    }
    out
}

pub fn has_refusal(findings: &[ComplianceFinding]) -> bool {
    findings.iter().any(|f| f.level == ComplianceLevel::Refuse)
}

fn refuse(message: &str, next_action: &str) -> ComplianceFinding {
    ComplianceFinding {
        level: ComplianceLevel::Refuse,
        message: message.into(),
        next_action: next_action.into(),
    }
}

fn warn(message: &str, next_action: &str) -> ComplianceFinding {
    ComplianceFinding {
        level: ComplianceLevel::Warn,
        message: message.into(),
        next_action: next_action.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cocli::slurp::SlurpConfig;

    #[test]
    fn refuses_unknown_multi_worker() {
        let cfg = SlurpConfig::from_toml_str(
            &SlurpConfig::sample().replace("kind = \"colab-paid\"", "kind = \"unknown\""),
        )
        .unwrap();
        assert!(has_refusal(&validate_slurp(&cfg)));
    }

    #[test]
    fn refuses_fallback_for_unknown() {
        let cfg = SlurpConfig::from_toml_str(
            &SlurpConfig::sample()
                .replace("kind = \"colab-paid\"", "kind = \"unknown\"")
                .replace(
                    "accelerator = \"L4\"",
                    "accelerator = \"L4\"\nallow_fallback_account = true",
                ),
        )
        .unwrap();
        assert!(
            validate_slurp(&cfg)
                .iter()
                .any(|f| f.message.contains("fallback"))
        );
    }
}
