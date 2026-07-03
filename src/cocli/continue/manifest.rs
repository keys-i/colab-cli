//! Stable JSON protocol types shared by `colab-cli` crates.

use serde::{Deserialize, Serialize};

pub const CONTINUATION_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRef {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountInfo {
    pub kind: String,
    pub path: String,
    pub mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub mtime_unix: u64,
    #[serde(default)]
    pub executable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub id: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GitSnapshot {
    #[serde(default)]
    pub commit_hash: Option<String>,
    pub dirty_tree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationManifest {
    pub version: u32,
    pub created_at: String,
    pub session: SessionRef,
    pub runtime_class: String,
    #[serde(default)]
    pub accelerator_type: Option<String>,
    #[serde(default)]
    pub notebook_or_script_path: Option<String>,
    #[serde(default)]
    pub command_args: Vec<String>,
    #[serde(default)]
    pub env_restore_plan: Vec<String>,
    #[serde(default)]
    pub installed_packages_snapshot: Vec<String>,
    #[serde(default)]
    pub mounts: Vec<MountInfo>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub executed_steps: Vec<ExecutionStep>,
    #[serde(default)]
    pub pending_steps: Vec<ExecutionStep>,
    #[serde(default)]
    pub stdout_refs: Vec<String>,
    #[serde(default)]
    pub stderr_refs: Vec<String>,
    #[serde(default)]
    pub log_refs: Vec<String>,
    #[serde(default)]
    pub random_seed_metadata: Vec<String>,
    #[serde(default)]
    pub git: GitSnapshot,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub fleet_name: Option<String>,
    #[serde(default)]
    pub shard_id: Option<String>,
}

impl ContinuationManifest {
    pub fn new(created_at: impl Into<String>, session_name: impl Into<String>) -> Self {
        Self {
            version: CONTINUATION_VERSION,
            created_at: created_at.into(),
            session: SessionRef {
                id: None,
                name: session_name.into(),
            },
            runtime_class: "unknown".to_string(),
            accelerator_type: None,
            notebook_or_script_path: None,
            command_args: Vec::new(),
            env_restore_plan: Vec::new(),
            installed_packages_snapshot: Vec::new(),
            mounts: Vec::new(),
            files: Vec::new(),
            artifacts: Vec::new(),
            executed_steps: Vec::new(),
            pending_steps: Vec::new(),
            stdout_refs: Vec::new(),
            stderr_refs: Vec::new(),
            log_refs: Vec::new(),
            random_seed_metadata: Vec::new(),
            git: GitSnapshot::default(),
            warnings: vec![
                "continuation restores files, metadata, and pending commands; it does not move live Python process memory".to_string(),
            ],
            fleet_name: None,
            shard_id: None,
        }
    }

    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Network,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub risk: RiskLevel,
    pub dry_run: bool,
    pub requires_session: bool,
    pub requires_network: bool,
    pub destructive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool: String,
    pub status: String,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default)]
    pub audit: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_roundtrips() {
        let mut manifest = ContinuationManifest::new("2026-07-03T00:00:00Z", "trainer");
        manifest.pending_steps.push(ExecutionStep {
            id: "train".into(),
            command: vec!["python".into(), "train.py".into()],
            cwd: Some("/content".into()),
        });

        let json = manifest.to_json_pretty().unwrap();
        let parsed = ContinuationManifest::from_json(json.as_bytes()).unwrap();
        assert_eq!(parsed, manifest);
        assert!(json.contains("does not move live Python process memory"));
    }
}
