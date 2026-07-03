use std::process::Command;

use clap::Parser;
use colab_cli::cocli::cli::args::Cli;

#[test]
fn parses_major_command_spaces() {
    for args in [
        ["colab-cli", "auth", "list"].as_slice(),
        ["colab-cli", "session", "last"].as_slice(),
        ["colab-cli", "exec", "last", "--confirm"].as_slice(),
        ["colab-cli", "fs", "changed", ".", "/content"].as_slice(),
        ["colab-cli", "mount", "list"].as_slice(),
        ["colab-cli", "env", "freeze"].as_slice(),
        ["colab-cli", "runtime", "fit", "--model", "llama-7b"].as_slice(),
        ["colab-cli", "slurp", "schema"].as_slice(),
        ["colab-cli", "fleet", "plan"].as_slice(),
        ["colab-cli", "tools", "list"].as_slice(),
        ["colab-cli", "agent", "tools"].as_slice(),
        ["colab-cli", "continue", "last"].as_slice(),
        ["colab-cli", "config", "path"].as_slice(),
        ["colab-cli", "doctor", "quick"].as_slice(),
        ["colab-cli", "release", "name", "v0.4.2"].as_slice(),
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    }
}

#[test]
fn json_output_has_no_ansi() {
    let out = bin().args(["--json", "doctor", "quick"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("\x1b["));
    assert!(stdout.contains("next_action"));
}

#[test]
fn quiet_suppresses_vibe_art() {
    let out = bin()
        .args(["--quiet", "doctor", "--vibe"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("/\\_/\\"));
}

#[test]
fn docs_exist() {
    for path in [
        "docs/refactor-map.md",
        "docs/prune-report.md",
        "docs/easter-eggs.md",
    ] {
        assert!(std::path::Path::new(path).exists(), "{path}");
    }
}

#[test]
fn config_open_prints_path_without_editor() {
    let out = bin()
        .env_remove("EDITOR")
        .args(["config", "open"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("config.toml"));
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_colab-cli"))
}
