use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: &'static str,
    pub ok: bool,
    pub next_action: &'static str,
}
