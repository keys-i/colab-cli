//! Built-in tool registry for `colab-cli`.
//!
//! The registry is intentionally enum-driven. External plugins can wrap these
//! specs without forcing the core CLI to own an async trait object stack.

use cocli_protocol::{RiskLevel, ToolOutput, ToolSpec};
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
    SessionStatus,
    ExecPython,
    ExecNotebook,
    FsList,
    FsPush,
    FsPull,
    EnvInstall,
    ContinueSave,
    ContinueResume,
    RuntimeInfo,
    Doctor,
}

impl BuiltinTool {
    pub const ALL: [Self; 12] = [
        Self::SessionNew,
        Self::SessionStatus,
        Self::ExecPython,
        Self::ExecNotebook,
        Self::FsList,
        Self::FsPush,
        Self::FsPull,
        Self::EnvInstall,
        Self::ContinueSave,
        Self::ContinueResume,
        Self::RuntimeInfo,
        Self::Doctor,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::SessionNew => "session_new",
            Self::SessionStatus => "session_status",
            Self::ExecPython => "exec_python",
            Self::ExecNotebook => "exec_notebook",
            Self::FsList => "fs_list",
            Self::FsPush => "fs_push",
            Self::FsPull => "fs_pull",
            Self::EnvInstall => "env_install",
            Self::ContinueSave => "continue_save",
            Self::ContinueResume => "continue_resume",
            Self::RuntimeInfo => "runtime_info",
            Self::Doctor => "doctor",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == name)
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
            Self::SessionStatus => (
                "inspect a Colab session",
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
            Self::EnvInstall => (
                "install packages into a session",
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
            Self::Doctor => ("run local diagnostics", RiskLevel::Low, false, false, false),
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
            Self::SessionStatus => session_cmd("session", "status", get("session")),
            Self::ExecPython => session_cmd("exec", "py", get("session")),
            Self::ExecNotebook => session_cmd("exec", "nb", get("session")),
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
            Self::EnvInstall => session_cmd("env", "install", get("session")),
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
            Self::RuntimeInfo => vec!["runtime".into(), "info".into()],
            Self::Doctor => vec!["doctor".into()],
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
        assert!(names.contains(&"session_new".into()));
        assert!(names.contains(&"continue_resume".into()));
        assert!(names.contains(&"doctor".into()));
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
