//! Built-in tool registry for `colab-cli`.
//!
//! The registry is intentionally enum-driven. External plugins can wrap these
//! specs without forcing the core CLI to own an async trait object stack.

use crate::cocli::r#continue::manifest::{RiskLevel, ToolOutput, ToolSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("unknown tool: {0}")]
    Unknown(String),
    #[error("tool {tool} is destructive; pass confirm=true")]
    ConfirmationRequired { tool: String },
}

pub type Result<T> = std::result::Result<T, ToolError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinTool {
    SessionNew,
    SessionList,
    SessionUrl,
    SessionStatus,
    RunScript,
    ExecPython,
    ExecNotebook,
    RunInstall,
    FsList,
    FsPush,
    FsPull,
    FsSync,
    EnvInstall,
    DriveStatus,
    DriveMount,
    ContinueSave,
    ContinueResume,
    RuntimeInfo,
    StatusRuntime,
    Doctor,
    SlurpPlan,
    SlurpRun,
    FleetPlan,
    FleetStatus,
    AgentPlan,
    AgentAudit,
}

impl BuiltinTool {
    pub const ALL: [Self; 24] = [
        Self::SessionNew,
        Self::SessionList,
        Self::SessionUrl,
        Self::SessionStatus,
        Self::RunScript,
        Self::ExecPython,
        Self::ExecNotebook,
        Self::RunInstall,
        Self::FsList,
        Self::FsPush,
        Self::FsPull,
        Self::FsSync,
        Self::DriveStatus,
        Self::DriveMount,
        Self::ContinueSave,
        Self::ContinueResume,
        Self::StatusRuntime,
        Self::Doctor,
        Self::SlurpPlan,
        Self::SlurpRun,
        Self::FleetPlan,
        Self::FleetStatus,
        Self::AgentPlan,
        Self::AgentAudit,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::SessionNew => "session.new",
            Self::SessionList => "session.list",
            Self::SessionUrl => "session.url",
            Self::SessionStatus => "session.status",
            Self::RunScript => "run.script",
            Self::ExecPython => "run.python",
            Self::ExecNotebook => "run.notebook",
            Self::RunInstall => "run.install",
            Self::FsList => "fs.list",
            Self::FsPush => "fs.push",
            Self::FsPull => "fs.pull",
            Self::FsSync => "fs.sync",
            Self::EnvInstall => "run.install",
            Self::DriveStatus => "fs.drive.status",
            Self::DriveMount => "fs.drive.mount",
            Self::ContinueSave => "continue.save",
            Self::ContinueResume => "continue.resume",
            Self::RuntimeInfo => "runtime.info",
            Self::StatusRuntime => "status.runtime",
            Self::Doctor => "status.check",
            Self::SlurpPlan => "slurp.plan",
            Self::SlurpRun => "slurp.run",
            Self::FleetPlan => "fleet.plan",
            Self::FleetStatus => "fleet.status",
            Self::AgentPlan => "agent.plan",
            Self::AgentAudit => "agent.audit",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|t| t.name() == name || t.legacy_name() == name)
    }

    fn legacy_name(self) -> &'static str {
        match self {
            Self::SessionNew => "session_new",
            Self::SessionList => "session_list",
            Self::SessionUrl => "session_url",
            Self::SessionStatus => "session_status",
            Self::RunScript => "exec_script",
            Self::ExecPython => "exec_python",
            Self::ExecNotebook => "exec_notebook",
            Self::RunInstall => "env_install",
            Self::FsList => "fs_list",
            Self::FsPush => "fs_push",
            Self::FsPull => "fs_pull",
            Self::FsSync => "fs_sync",
            Self::EnvInstall => "env_install",
            Self::DriveStatus => "drive_status",
            Self::DriveMount => "drive_mount",
            Self::ContinueSave => "continue_save",
            Self::ContinueResume => "continue_resume",
            Self::RuntimeInfo => "runtime_info",
            Self::StatusRuntime => "runtime_info",
            Self::Doctor => "doctor",
            Self::SlurpPlan => "slurp_plan",
            Self::SlurpRun => "slurp_run",
            Self::FleetPlan => "fleet_plan",
            Self::FleetStatus => "fleet_status",
            Self::AgentPlan => "agent_plan",
            Self::AgentAudit => "agent_audit",
        }
    }

    pub fn spec(self) -> ToolSpec {
        let (description, risk, requires_session, requires_network, destructive) = match self {
            Self::SessionNew => (
                "create a Colab session",
                RiskLevel::Network,
                false,
                true,
                false,
            ),
            Self::SessionList => (
                "list known sessions",
                RiskLevel::Network,
                false,
                true,
                false,
            ),
            Self::SessionUrl => ("print a session URL", RiskLevel::Low, true, false, false),
            Self::SessionStatus => (
                "inspect a Colab session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::RunScript => (
                "run a script in a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::ExecPython => (
                "execute Python code in a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::ExecNotebook => (
                "execute a notebook in a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::FsList => (
                "list files in a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::FsPush => (
                "copy local files into a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::FsPull => (
                "copy files from a session to local disk",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::RunInstall | Self::EnvInstall => (
                "install packages into a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::FsSync => (
                "plan file sync changes",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::DriveStatus => (
                "check Google Drive mount state",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::DriveMount => (
                "mount Google Drive in a session",
                RiskLevel::Network,
                true,
                true,
                false,
            ),
            Self::ContinueSave => (
                "write a continuation manifest",
                RiskLevel::Low,
                true,
                false,
                false,
            ),
            Self::ContinueResume => (
                "restore files and replay pending continuation steps",
                RiskLevel::Network,
                false,
                true,
                false,
            ),
            Self::RuntimeInfo => (
                "inspect runtime metadata",
                RiskLevel::Low,
                false,
                false,
                false,
            ),
            Self::StatusRuntime => ("show runtime status", RiskLevel::Low, false, false, false),
            Self::Doctor => ("run local diagnostics", RiskLevel::Low, false, false, false),
            Self::SlurpPlan => ("explain a Slurp plan", RiskLevel::Low, false, false, false),
            Self::SlurpRun => (
                "run a Slurp workflow after confirmation",
                RiskLevel::Network,
                false,
                true,
                false,
            ),
            Self::FleetPlan => (
                "plan approved runtimes",
                RiskLevel::Network,
                false,
                true,
                false,
            ),
            Self::FleetStatus => (
                "show fleet planning status",
                RiskLevel::Low,
                false,
                false,
                false,
            ),
            Self::AgentPlan => (
                "draft an explicit agent plan",
                RiskLevel::Low,
                false,
                false,
                false,
            ),
            Self::AgentAudit => ("audit an agent plan", RiskLevel::Low, false, false, false),
        };

        ToolSpec {
            name: self.name().to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": true
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "required": ["tool", "status", "data", "audit"]
            }),
            risk,
            dry_run: true,
            requires_session,
            requires_network,
            destructive,
        }
    }

    pub fn cli_command(self, input: &serde_json::Value) -> Vec<String> {
        let get = |key: &str| input.get(key).and_then(serde_json::Value::as_str);
        match self {
            Self::SessionNew => {
                let mut cmd = vec!["session".into(), "new".into()];
                if let Some(name) = get("name") {
                    cmd.extend(["--name".into(), name.into()]);
                }
                cmd
            }
            Self::SessionList => vec!["session".into(), "list".into()],
            Self::SessionUrl => session_cmd("session", "url", get("session")),
            Self::SessionStatus => session_cmd("status", "session", get("session")),
            Self::RunScript => session_cmd("run", "script", get("session")),
            Self::ExecPython => session_cmd("run", "py", get("session")),
            Self::ExecNotebook => session_cmd("run", "notebook", get("session")),
            Self::RunInstall | Self::EnvInstall => session_cmd("run", "install", get("session")),
            Self::FsList => vec![
                "fs".into(),
                "ls".into(),
                get("path").unwrap_or("/content").into(),
            ],
            Self::FsPush => vec![
                "fs".into(),
                "push".into(),
                get("src").unwrap_or(".").into(),
                get("dest").unwrap_or("/content").into(),
            ],
            Self::FsPull => vec![
                "fs".into(),
                "pull".into(),
                get("src").unwrap_or("/content").into(),
                get("dest").unwrap_or(".").into(),
            ],
            Self::FsSync => vec![
                "fs".into(),
                "sync".into(),
                ".".into(),
                "/content".into(),
                "--dry-run".into(),
            ],
            Self::DriveStatus => {
                let mut cmd = vec!["fs".into(), "drive".into(), "status".into()];
                if let Some(session) = get("session") {
                    cmd.extend(["--session".into(), session.into()]);
                }
                cmd
            }
            Self::DriveMount => {
                let mut cmd = vec!["fs".into(), "drive".into(), "mount".into()];
                if let Some(session) = get("session") {
                    cmd.extend(["--session".into(), session.into()]);
                }
                cmd
            }
            Self::ContinueSave => {
                let mut cmd = session_cmd("continue", "save", get("session"));
                if let Some(name) = get("name") {
                    cmd.extend(["--name".into(), name.into()]);
                }
                cmd
            }
            Self::ContinueResume => vec![
                "continue".into(),
                "resume".into(),
                get("name").unwrap_or("latest").into(),
            ],
            Self::RuntimeInfo => vec!["status".into(), "runtime".into()],
            Self::StatusRuntime => vec!["status".into(), "runtime".into(), "--all".into()],
            Self::Doctor => vec!["status".into(), "check".into()],
            Self::SlurpPlan => vec!["slurp".into(), "plan".into()],
            Self::SlurpRun => vec!["slurp".into(), "run".into(), "--dry-run".into()],
            Self::FleetPlan => vec!["fleet".into(), "plan".into(), "--cost".into()],
            Self::FleetStatus => vec!["status".into(), "fleet".into()],
            Self::AgentPlan => vec!["agent".into(), "plan".into()],
            Self::AgentAudit => vec!["agent".into(), "audit-plan".into()],
        }
    }
}

pub struct ToolRegistry;

impl ToolRegistry {
    pub fn specs() -> Vec<ToolSpec> {
        BuiltinTool::ALL
            .into_iter()
            .map(BuiltinTool::spec)
            .collect()
    }

    pub fn inspect(name: &str) -> Result<ToolSpec> {
        BuiltinTool::from_name(name)
            .map(BuiltinTool::spec)
            .ok_or_else(|| ToolError::Unknown(name.into()))
    }

    pub fn run_plan(name: &str, input: serde_json::Value, confirm: bool) -> Result<ToolOutput> {
        let tool = BuiltinTool::from_name(name).ok_or_else(|| ToolError::Unknown(name.into()))?;
        let spec = tool.spec();
        if spec.destructive && !confirm {
            return Err(ToolError::ConfirmationRequired {
                tool: name.to_string(),
            });
        }
        let command = tool.cli_command(&input);
        Ok(ToolOutput {
            tool: name.to_string(),
            status: "planned".to_string(),
            data: serde_json::json!({
                "command": command,
                "dry_run_supported": spec.dry_run,
                "input": input
            }),
            audit: vec![format!("planned tool {name}")],
        })
    }
}

fn session_cmd(space: &str, command: &str, session: Option<&str>) -> Vec<String> {
    let mut cmd = vec![space.into(), command.into()];
    if let Some(session) = session {
        cmd.extend(["--session".into(), session.into()]);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_requested_builtins() {
        let names: Vec<_> = ToolRegistry::specs().into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"session.new".into()));
        assert!(names.contains(&"continue.resume".into()));
        assert!(names.contains(&"status.check".into()));
        assert!(BuiltinTool::from_name("exec_python").is_some());
    }

    #[test]
    fn run_plan_is_json_serializable() {
        let out = ToolRegistry::run_plan(
            "fs_push",
            serde_json::json!({"src": "./data.csv", "dest": "/content/data.csv"}),
            false,
        )
        .unwrap();
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("fs"));
        assert!(json.contains("planned"));
    }
}
