use std::process::Command;

use clap::Parser;
use colab_cli::cocli::cli::args::{Cli, Commands, FsCommands, FsDriveCommands};

#[test]
fn parses_major_command_spaces() {
    for args in [
        ["colab-cli", "session", "last"].as_slice(),
        ["colab-cli", "run", "last", "--confirm"].as_slice(),
        ["colab-cli", "run", "py", "--code", "print(1)"].as_slice(),
        ["colab-cli", "run", "notebook", "report.ipynb"].as_slice(),
        ["colab-cli", "run", "repl"].as_slice(),
        ["colab-cli", "run", "shell"].as_slice(),
        ["colab-cli", "run", "pip", "install", "torch"].as_slice(),
        ["colab-cli", "run", "pip", "freeze"].as_slice(),
        ["colab-cli", "run", "pip", "restore", "requirements.txt"].as_slice(),
        ["colab-cli", "run", "pip", "check"].as_slice(),
        ["colab-cli", "run", "pip", "list"].as_slice(),
        ["colab-cli", "run", "ast", "file.py"].as_slice(),
        ["colab-cli", "run", "watch", "file.py", "--ast"].as_slice(),
        ["colab-cli", "run", "install", "torch"].as_slice(),
        ["colab-cli", "fs", "changed", ".", "/content"].as_slice(),
        ["colab-cli", "fs", "drive", "mount"].as_slice(),
        ["colab-cli", "fs", "drive", "mount", "--timeout", "180"].as_slice(),
        [
            "colab-cli",
            "fs",
            "drive",
            "mount",
            "--preflight-timeout",
            "10",
            "--retries",
            "2",
        ]
        .as_slice(),
        ["colab-cli", "fs", "drive", "status"].as_slice(),
        ["colab-cli", "fs", "drive", "list"].as_slice(),
        ["colab-cli", "status", "runtime", "--gpu"].as_slice(),
        ["colab-cli", "status", "runtime", "--tpu"].as_slice(),
        ["colab-cli", "status", "runtime", "--versions"].as_slice(),
        ["colab-cli", "status", "runtime", "--backend"].as_slice(),
        ["colab-cli", "status", "check"].as_slice(),
        ["colab-cli", "status", "version"].as_slice(),
        ["colab-cli", "distribute", "plan"].as_slice(),
        ["colab-cli", "distribute", "recipe", "explain"].as_slice(),
        ["colab-cli", "distribute", "pool", "plan"].as_slice(),
        ["colab-cli", "distribute", "shard", "plan"].as_slice(),
        ["colab-cli", "slurp", "explain"].as_slice(),
        ["colab-cli", "fleet", "plan"].as_slice(),
        ["colab-cli", "settings", "skills", "list"].as_slice(),
        ["colab-cli", "settings", "skills", "inspect", "recipe.plan"].as_slice(),
        ["colab-cli", "settings", "skills", "mcp"].as_slice(),
        ["colab-cli", "settings", "ui", "get"].as_slice(),
        ["colab-cli", "settings", "ui", "set", "animations", "false"].as_slice(),
        ["colab-cli", "settings", "ui", "preview"].as_slice(),
        ["colab-cli", "settings", "experiments"].as_slice(),
        ["colab-cli", "settings", "experiments", "get"].as_slice(),
        [
            "colab-cli",
            "settings",
            "experiments",
            "set",
            "distribute",
            "true",
        ]
        .as_slice(),
        ["colab-cli", "settings", "experiments", "reset"].as_slice(),
        ["colab-cli", "settings", "support", "bug-report"].as_slice(),
        ["colab-cli", "settings", "about"].as_slice(),
        ["colab-cli", "settings", "update", "check"].as_slice(),
        ["colab-cli", "settings", "update", "install", "--yes"].as_slice(),
        ["colab-cli", "settings", "billing", "open", "--dry-run"].as_slice(),
        ["colab-cli", "settings", "billing", "status"].as_slice(),
        ["colab-cli", "ai"].as_slice(),
        ["colab-cli", "ai", "tools"].as_slice(),
        ["colab-cli", "ai", "tools", "list"].as_slice(),
        ["colab-cli", "ai", "tools", "inspect", "recipe.plan"].as_slice(),
        ["colab-cli", "ai", "ast", "file.py"].as_slice(),
        ["colab-cli", "ai", "ast", "watch", "file.py"].as_slice(),
        ["colab-cli", "ai", "mcp"].as_slice(),
        ["colab-cli", "ai", "mcp", "serve", "--stdio"].as_slice(),
        ["colab-cli", "ai", "plan", "train a model"].as_slice(),
        ["colab-cli", "ai", "audit", "plan.toml"].as_slice(),
        ["colab-cli", "continue", "last"].as_slice(),
        ["colab-cli", "auth", "login", "--method", "adc"].as_slice(),
        ["colab-cli", "auth", "login", "--method", "oauth2"].as_slice(),
        ["colab-cli", "auth", "status"].as_slice(),
        ["colab-cli", "auth", "list"].as_slice(),
        ["colab-cli", "session", "refresh"].as_slice(),
        ["colab-cli", "session", "repair"].as_slice(),
        ["colab-cli", "session", "reconnect"].as_slice(),
        ["colab-cli", "session", "logs", "--tail", "50"].as_slice(),
        ["colab-cli", "session", "kernel", "status"].as_slice(),
        ["colab-cli", "session", "kernel", "restart", "--yes"].as_slice(),
        ["colab-cli", "settings", "path"].as_slice(),
        ["colab-cli", "settings", "locate"].as_slice(),
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    }
}

#[test]
fn drive_mount_timeout_default_allows_human_auth() {
    let cli = Cli::try_parse_from(["colab-cli", "fs", "drive", "mount"]).unwrap();
    let Some(Commands::Fs {
        command:
            FsCommands::Drive {
                command:
                    FsDriveCommands::Mount {
                        timeout,
                        preflight_timeout,
                        ..
                    },
            },
    }) = cli.command
    else {
        panic!("expected fs drive mount");
    };
    assert_eq!(timeout, 600);
    assert_eq!(preflight_timeout, 10);
}

#[test]
fn top_level_help_has_final_command_spaces() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    for name in [
        "session",
        "run",
        "fs",
        "status",
        "ai",
        "auth",
        "settings",
        "completions",
    ] {
        assert!(stdout.contains(name), "{name}");
    }
    for old in [
        "continue",
        "distribute",
        "slurp",
        "fleet",
        "exec",
        "env",
        "mount",
        "runtime",
        "tools",
        "config",
        "doctor",
        "release",
        "agent",
    ] {
        assert!(!stdout.contains(&format!("  {old}")), "{old}");
    }
    assert!(!stdout.contains("--color"));
    assert!(stdout.contains("--no-color"));
}

#[test]
fn verbose_count_parses_and_caps() {
    let cli = Cli::try_parse_from(["colab-cli", "-v", "status"]).unwrap();
    assert_eq!(cli.verbose, 1);
    let cli = Cli::try_parse_from(["colab-cli", "-vv", "status"]).unwrap();
    assert_eq!(cli.verbose, 2);
    let cli = Cli::try_parse_from(["colab-cli", "-vvv", "status"]).unwrap();
    assert_eq!(cli.verbose, 3);
    let cli = Cli::try_parse_from(["colab-cli", "--verbose", "--verbose", "status"]).unwrap();
    assert_eq!(cli.verbose, 2);
}

#[test]
fn verbose_goes_to_stderr_and_json_stays_clean() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["--json", "-v", "status"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
    assert!(!stdout.contains("debug1:"));
    assert!(!stdout.contains("\x1b["));
    assert!(stderr.contains("debug1: command status"));
    assert!(!stderr.contains("\x1b["));
}

#[test]
fn quiet_suppresses_verbose_debug() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["--quiet", "-vvv", "status"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(!stderr.contains("debug"));
}

#[test]
fn verbose_command_names_cover_major_families() {
    let home = tempfile::tempdir().unwrap();
    for (args, expected) in [
        (["-v", "status"].as_slice(), "debug1: command status"),
        (
            ["-v", "session", "list"].as_slice(),
            "debug1: command session.list",
        ),
        (
            ["-v", "ai", "tools", "list"].as_slice(),
            "debug1: command ai.tools",
        ),
        (
            ["-v", "settings", "path"].as_slice(),
            "debug1: command settings.path",
        ),
        (
            ["-v", "auth", "status"].as_slice(),
            "debug1: command auth.status",
        ),
        (
            ["-v", "run", "pip", "list"].as_slice(),
            "debug1: command run.pip.list",
        ),
        (
            ["-v", "fs", "drive", "status"].as_slice(),
            "debug1: command fs.drive.status",
        ),
    ] {
        let out = bin().env("HOME", home.path()).args(args).output().unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
    }
}

#[test]
fn hidden_aliases_parse_for_one_cycle() {
    for args in [
        ["colab-cli", "doctor"].as_slice(),
        ["colab-cli", "runtime", "gpu"].as_slice(),
        ["colab-cli", "tools", "list"].as_slice(),
        ["colab-cli", "config", "path"].as_slice(),
        ["colab-cli", "env", "install", "torch"].as_slice(),
        ["colab-cli", "exec", "py", "--code", "print(1)"].as_slice(),
        ["colab-cli", "mount", "drive"].as_slice(),
        ["colab-cli", "log"].as_slice(),
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    }
}

#[test]
fn json_output_has_no_ansi() {
    let out = bin().args(["--json", "status", "quick"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("\x1b["));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok());
    assert!(stdout.contains("fix"));
}

#[test]
fn status_human_output_is_not_json() {
    let out = bin().arg("status").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("cocli status"));
    assert!(stdout.contains("Auth"));
    assert!(!stdout.trim_start().starts_with('{'));
    assert!(!stdout.contains("Quick Actions"));
    assert!(!stdout.contains("\nNext\n"));
}

#[test]
fn no_command_shows_launcher_fallback_in_non_tty() {
    let out = bin().output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Google Colab from the terminal"));
    assert!(stdout.contains("Usage: colab-cli [OPTIONS] <COMMAND>"));
    assert!(!stdout.contains("Quick actions"));
    assert!(!stdout.contains("command preview"));
}

#[test]
fn quiet_suppresses_vibe_art() {
    let out = bin().args(["--quiet", "doctor"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("moved:"));
}

#[test]
fn docs_exist() {
    for path in [
        "docs/refactor-map.md",
        "docs/prune-report.md",
        "docs/easter-eggs.md",
        "docs/research.md",
        "docs/command-audit.md",
        "docs/drive.md",
        "docs/ui.md",
        "docs/settings.md",
        "docs/skills.md",
        "docs/maintainer.md",
        "docs/feature-test-plan.md",
        "docs/live-testing.md",
        "docs/google-colab-cli-map.md",
        "docs/colabtools-feature-map.md",
        "docs/debugging.md",
        "docs/troubleshooting.md",
        "docs/auth.md",
        "docs/logs.md",
        "docs/run.md",
        "plan.md",
    ] {
        assert!(std::path::Path::new(path).exists(), "{path}");
    }
}

#[test]
fn config_open_prints_path_without_editor() {
    let out = bin()
        .env_remove("EDITOR")
        .args(["settings", "edit"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("config.toml"));
}

#[test]
fn settings_skills_list_is_catalog_not_debug_rows() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["settings", "skills", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Tool"));
    assert!(stdout.contains("recipe.plan"));
    assert!(stdout.contains("recipe.explain"));
    assert!(stdout.contains("distribute.plan"));
    assert!(stdout.contains("mcp.tools"));
    assert!(stdout.contains("agent.audit"));
    assert!(!stdout.contains("continue.resume"));
    assert!(!stdout.contains("session.new"));
    assert!(!stdout.contains("run.python"));
    assert!(!stdout.contains("fs.push"));
    assert!(!stdout.contains("session_new"));
    assert!(!stdout.contains("session=false"));
    assert!(!stdout.contains("enter inspect"));
    assert!(!stdout.contains("/ search"));
}

#[test]
fn settings_skills_json_has_stable_fields() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["--json", "settings", "skills", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("\x1b["));
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let first = value.as_array().unwrap().first().unwrap();
    assert!(first.get("name").is_some());
    assert!(first.get("category").is_some());
    assert!(first.get("scope").is_some());
    assert!(first.get("risk").is_some());
    assert!(first.get("needs_session").is_some());
    assert!(first.get("state").is_some());
}

#[test]
fn settings_default_renders_sections() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .arg("settings")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Settings"));
    assert!(stdout.contains("General"));
    assert!(stdout.contains("UI"));
    assert!(stdout.contains("Experiments"));
    assert!(stdout.contains("AI"));
    assert!(stdout.contains("Auth"));
    assert!(stdout.contains("Billing"));
    assert!(!stdout.contains("Dev"));
    assert!(!stdout.contains("Quick Actions"));
    assert!(!stdout.trim_start().starts_with('{'));
}

#[test]
fn settings_experiments_default_off_and_persist() {
    let home = tempfile::tempdir().unwrap();
    let list = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments"])
        .output()
        .unwrap();
    assert!(list.status.success());
    let stdout = String::from_utf8(list.stdout).unwrap();
    assert!(stdout.contains("[ ] Multi-login"));
    assert!(stdout.contains("[ ] Continue"));
    assert!(stdout.contains("[ ] Distribute"));
    assert!(stdout.contains("[ ] MCP server"));
    assert!(stdout.contains("[ ] AI plan runner"));
    assert!(stdout.contains("[ ] AST observer"));
    assert!(!stdout.contains("Quick Actions"));

    let set = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments", "set", "mcp-server", "true"])
        .output()
        .unwrap();
    assert!(set.status.success());

    let get = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments", "get", "mcp-server"])
        .output()
        .unwrap();
    assert!(get.status.success());
    assert_eq!(String::from_utf8(get.stdout).unwrap().trim(), "true");

    let blocked = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments", "set", "multi-login", "true"])
        .output()
        .unwrap();
    assert!(!blocked.status.success());
    let stderr = String::from_utf8(blocked.stderr).unwrap();
    assert!(stderr.contains("multi-login requires distribute"));

    let distribute = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments", "set", "distribute", "true"])
        .output()
        .unwrap();
    assert!(distribute.status.success());
    let multi = bin()
        .env("HOME", home.path())
        .args(["settings", "experiments", "set", "multi-login", "true"])
        .output()
        .unwrap();
    assert!(multi.status.success());
}

#[test]
fn settings_ui_set_persists_in_temp_config() {
    let home = tempfile::tempdir().unwrap();
    let set = bin()
        .env("HOME", home.path())
        .args(["settings", "ui", "set", "animations", "false"])
        .output()
        .unwrap();
    assert!(set.status.success());

    let get = bin()
        .env("HOME", home.path())
        .args(["settings", "ui", "get", "animations"])
        .output()
        .unwrap();
    assert!(get.status.success());
    let stdout = String::from_utf8(get.stdout).unwrap();
    assert_eq!(stdout.trim(), "false");
}

#[test]
fn ai_tools_list_is_agent_catalog() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["ai", "tools", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("AI tools"));
    assert!(stdout.contains("Agent-facing workflows"));
    assert!(stdout.contains("recipe.plan"));
    assert!(stdout.contains("distribute.plan"));
    assert!(stdout.contains("fs.changed"));
    assert!(stdout.contains("ast.outline"));
    assert!(stdout.contains("mcp.tools"));
    assert!(stdout.contains("State"));
    assert!(stdout.contains("gated"));
    assert!(stdout.contains("off"));
    assert!(!stdout.contains("continue.resume"));
    assert!(!stdout.contains("session.new"));
    assert!(!stdout.contains("session=false"));
    assert!(!stdout.contains("Quick Actions"));
}

#[test]
fn ai_tools_json_is_clean() {
    let home = tempfile::tempdir().unwrap();
    let out = bin()
        .env("HOME", home.path())
        .args(["--json", "ai", "tools", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("\x1b["));
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        value
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"] == "recipe.plan")
    );
}

#[test]
fn optional_commands_are_experiment_gated() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        ["distribute", "plan"].as_slice(),
        ["continue", "last"].as_slice(),
        ["run", "ast", "Cargo.toml"].as_slice(),
    ] {
        let out = bin().env("HOME", home.path()).args(args).output().unwrap();
        assert!(!out.status.success(), "{args:?}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(stderr.contains("experimental feature disabled"), "{stderr}");
    }
}

#[test]
fn enabled_distribute_status_and_ast_outline_work() {
    let home = tempfile::tempdir().unwrap();
    let sample = home.path().join("sample.py");
    std::fs::write(
        &sample,
        "import os\nfrom pathlib import Path\n\nclass Job:\n    pass\n\ndef main():\n    return Path('.')\n\nif __name__ == '__main__':\n    main()\n",
    )
    .unwrap();

    assert!(
        bin()
            .env("HOME", home.path())
            .args(["settings", "experiments", "set", "distribute", "true"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let distribute = bin()
        .env("HOME", home.path())
        .args(["distribute", "status", "--json"])
        .output()
        .unwrap();
    assert!(distribute.status.success());
    let distribute_json: serde_json::Value = serde_json::from_slice(&distribute.stdout).unwrap();
    assert_eq!(distribute_json["enabled"], true);

    assert!(
        bin()
            .env("HOME", home.path())
            .args(["settings", "experiments", "set", "ast-observer", "true"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let ast = bin()
        .env("HOME", home.path())
        .args(["run", "ast", sample.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(ast.status.success());
    let stdout = String::from_utf8(ast.stdout).unwrap();
    assert!(!stdout.contains("\x1b["));
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        value["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "os")
    );
    assert!(
        value["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "main")
    );
    assert!(
        value["classes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "Job")
    );
    assert_eq!(value["main_guard"], true);
}

#[test]
fn ai_mcp_and_run_are_experiment_gated() {
    let home = tempfile::tempdir().unwrap();
    let mcp = bin()
        .env("HOME", home.path())
        .args(["ai", "mcp"])
        .output()
        .unwrap();
    assert!(!mcp.status.success());
    let stderr = String::from_utf8(mcp.stderr).unwrap();
    assert!(stderr.contains("experimental feature disabled"));
    assert!(stderr.contains("enable: colab-cli settings experiments"));

    let plan = home.path().join("plan.toml");
    std::fs::write(&plan, "confirm_required = true\n").unwrap();
    let run = bin()
        .env("HOME", home.path())
        .args(["ai", "run", plan.to_str().unwrap(), "--confirm"])
        .output()
        .unwrap();
    assert!(!run.status.success());
    let stderr = String::from_utf8(run.stderr).unwrap();
    assert!(stderr.contains("experimental feature disabled"));
}

#[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
#[test]
fn dev_release_is_maintainer_gated() {
    let home = tempfile::tempdir().unwrap();
    let blocked = bin()
        .env("HOME", home.path())
        .env("USER", "not-keys")
        .env("COLAB_CLI_OWNER", "keys")
        .env_remove("COLAB_CLI_MAINTAINER")
        .args(["settings", "dev", "release", "name"])
        .output()
        .unwrap();
    assert!(!blocked.status.success());
    let stderr = String::from_utf8(blocked.stderr).unwrap();
    assert!(stderr.contains("private maintainer command"));

    let allowed = bin()
        .env("HOME", home.path())
        .env("USER", "not-keys")
        .env("COLAB_CLI_OWNER", "keys")
        .env("COLAB_CLI_DEV", "1")
        .env("COLAB_CLI_MAINTAINER", "1")
        .args(["settings", "dev", "release", "name"])
        .output()
        .unwrap();
    assert!(allowed.status.success());
    let stdout = String::from_utf8(allowed.stdout).unwrap();
    assert!(stdout.trim_start().starts_with('v'));
    assert!(stdout.contains(" - "));
}

#[test]
fn fs_sync_json_dry_run_has_no_human_prefix() {
    let temp = std::env::temp_dir().join(format!("cocli-test-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(temp.join("a.txt"), "ok").unwrap();
    let out = bin()
        .args([
            "--json",
            "fs",
            "sync",
            temp.to_str().unwrap(),
            "/content/tmp",
            "--dry-run",
            "--explain",
        ])
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&temp);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.trim_start().starts_with('{'));
    assert!(!stdout.contains("sync dry-run planned"));
    assert!(!stdout.contains("\x1b["));
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_colab-cli"))
}
