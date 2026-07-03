use serde::{Deserialize, Serialize};

use crate::cocli::slurp::SlurpConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetPlan {
    pub name: String,
    pub requested_runtimes: u32,
    pub shard_count: u32,
    pub max_parallel_tasks: u32,
    pub budget_limit: String,
    pub stop_condition: String,
    pub data_movement: Vec<String>,
    pub warnings: Vec<String>,
    pub fast_path: String,
    pub assignments: Vec<TaskAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAssignment {
    pub shard: u32,
    pub account: String,
    pub accelerator: Option<String>,
    pub priority: u8,
}

pub fn plan(cfg: &SlurpConfig) -> FleetPlan {
    let requested_runtimes = cfg
        .accounts
        .iter()
        .map(|a| a.max_runtimes)
        .sum::<u32>()
        .max(1);
    let shard_count = estimate_shards(cfg);
    let mut assignments = Vec::with_capacity(shard_count as usize);
    let mut runtime_slots = Vec::new();
    for account in &cfg.accounts {
        for _ in 0..account.max_runtimes.max(1) {
            runtime_slots.push((account.name.clone(), account.accelerator.clone()));
        }
    }
    if runtime_slots.is_empty() {
        runtime_slots.push(("default".into(), None));
    }
    for shard in 0..shard_count {
        let slot = &runtime_slots[shard as usize % runtime_slots.len()];
        assignments.push(TaskAssignment {
            shard,
            account: slot.0.clone(),
            accelerator: slot.1.clone(),
            priority: if shard == 0 { 0 } else { 5 },
        });
    }

    let mut warnings = Vec::new();
    if cfg.files.push.iter().any(|p| p.contains("drive")) {
        warnings.push("Drive mount may be a data hot path; cache hot files on runtime disk".into());
    }
    if cfg.model.quant.as_deref() == Some("auto") {
        warnings.push("quantization plan is automatic; probe memory before large batches".into());
    }

    FleetPlan {
        name: cfg.slurp.name.clone(),
        requested_runtimes,
        shard_count,
        max_parallel_tasks: requested_runtimes.min(shard_count),
        budget_limit: format!(
            "{} runtime minutes / {} budget units",
            cfg.budget.max_runtime_minutes, cfg.budget.max_compute_units
        ),
        stop_condition: if cfg.budget.stop_on_budget {
            "stop when budget is reached".into()
        } else {
            "warn when budget is reached".into()
        },
        data_movement: cfg
            .files
            .push
            .iter()
            .map(|p| format!("push {p}"))
            .chain(cfg.files.pull.iter().map(|p| format!("pull {p}")))
            .collect(),
        warnings,
        fast_path: "smaller shards, checkpoint every shard, avoid Drive for hot inputs".into(),
        assignments,
    }
}

fn estimate_shards(cfg: &SlurpConfig) -> u32 {
    if cfg.work.shard_by.as_deref() == Some("lines") {
        16
    } else {
        1
    }
}

pub fn within_budget(plan: &FleetPlan, max_compute_units: u32) -> bool {
    plan.requested_runtimes <= max_compute_units
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cocli::slurp::SlurpConfig;

    #[test]
    fn first_fit_scheduler_respects_runtime_count() {
        let cfg = SlurpConfig::from_toml_str(SlurpConfig::sample()).unwrap();
        let plan = plan(&cfg);
        assert_eq!(plan.requested_runtimes, 3);
        assert!(plan.assignments.iter().all(|a| a.shard < plan.shard_count));
        assert!(within_budget(&plan, cfg.budget.max_compute_units));
    }
}
