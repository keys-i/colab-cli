use std::process::Command;

use clap::Parser;
use colab_cli::cocli::cli::args::Cli;

#[test]
fn parses_major_command_spaces() {
    for args in [
        ["colab-cli", "session", "last"].as_slice(),
        ["colab-cli", "run", "last", "--confirm"].as_slice(),
        ["colab-cli", "run", "py", "--code", "print(1)"].as_slice(),
        ["colab-cli", "run", "notebook", "report.ipynb"].as_slice(),
        ["colab-cli", "run", "install", "torch"].as_slice(),
        ["colab-cli", "fs", "changed", ".", "/content"].as_slice(),
        ["colab-cli", "fs", "drive", "mount"].as_slice(),
        ["colab-cli", "fs", "drive", "mount", "--timeout", "120"].as_slice(),
        ["colab-cli", "fs", "drive", "status"].as_slice(),
        ["colab-cli", "fs", "drive", "list"].as_slice(),
        ["colab-cli", "status", "runtime", "--gpu"].as_slice(),
        ["colab-cli", "status", "runtime", "--tpu"].as_slice(),
        ["colab-cli", "status", "runtime", "--versions"].as_slice(),
        ["colab-cli", "status", "runtime", "--backend"].as_slice(),
        ["colab-cli", "status", "check"].as_slice(),
        ["colab-cli", "slurp", "explain"].as_slice(),
        ["colab-cli", "fleet", "plan"].as_slice(),
        ["colab-cli", "settings", "skills", "list"].as_slice(),
        ["colab-cli", "settings", "skills", "inspect", "session.new"].as_slice(),
        ["colab-cli", "continue", "last"].as_slice(),
        ["colab-cli", "settings", "path"].as_slice(),
        ["colab-cli", "settings", "locate"].as_slice(),
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    }
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
        "continue",
        "slurp",
        "fleet",
        "auth",
        "settings",
        "completions",
    ] {
        assert!(stdout.contains(name), "{name}");
    }
    for old in [
        "exec", "env", "mount", "runtime", "tools", "config", "doctor", "release", "agent",
    ] {
        assert!(!stdout.contains(&format!("  {old}")), "{old}");
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
    assert!(stdout.contains("next_action"));
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
        "docs/feature-test-plan.md",
        "docs/live-testing.md",
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
    let out = bin().args(["settings", "skills", "list"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Skill"));
    assert!(stdout.contains("session.new"));
    assert!(stdout.contains("run.python"));
    assert!(!stdout.contains("session_new"));
    assert!(!stdout.contains("session=false"));
}

#[test]
fn settings_skills_json_has_stable_fields() {
    let out = bin()
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
    assert!(first.get("risk").is_some());
    assert!(first.get("needs_session").is_some());
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
