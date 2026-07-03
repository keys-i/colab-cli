use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{CommandFactory, Parser};

use crate::cocli::auth;
use crate::cocli::cli::{
    AgentCommands, AuthCommands, AuthProfileArgs, Cli, Commands, CompatTransferArgs,
    ConfigCommands, ContinueCommands, EnvCommands, ExecCommands, FileCommands, FleetCommands,
    FleetConfigArgs, FsCommands, FsDiffArgs, FsDriveCommands, FsSyncArgs, MountCommands,
    ReleaseCommands, RunCommands, RuntimeCommands, ServerCommands, SessionCommands, SessionNameArg,
    SessionNewArgs, SettingsCommands, SkillCommands, SlurpCommands, StatusCommands, ToolsCommands,
};
use crate::cocli::config::{self, ColabConfig};
use crate::cocli::error::{ColabError, Result};
use crate::cocli::exec::runner;
use crate::cocli::session::ServerManager;
use crate::cocli::session::client::ColabClient;
use crate::cocli::session::model::{Shape, Variant};
use crate::cocli::session::store::StoredServer;
use crate::cocli::ui::Ui;

pub async fn main_entry() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let color_choice: config::ColorChoice = cli.color.parse().unwrap_or_default();
    let use_color = color_choice.enabled(
        cli.no_color || std::env::var_os("NO_COLOR").is_some(),
        std::env::var_os("CI").is_some(),
        cli.quiet,
        cli.json,
    );
    colored::control::set_override(use_color);
    let ring_bell =
        config::terminal_bell_allowed(cli.bell, std::env::var_os("CI").is_some(), cli.quiet);
    let json_mode = cli.json;
    let verbose = cli.verbose;
    let ui = Ui::new(cli.quiet || cli.json);

    if let Err(e) = run(cli, ui).await {
        if json_mode {
            print_error_json(&e, verbose);
        } else {
            ui.error(&e.to_string());
            if let ColabError::Drive {
                next_action, raw, ..
            } = &e
            {
                if let Some(next) = next_action {
                    eprintln!("  next: {next}");
                }
                if verbose && let Some(raw) = raw {
                    eprintln!("\n{raw}");
                }
            }
        }
        if ring_bell {
            eprint!("\x07");
        }

        if !json_mode {
            match &e {
                ColabError::NotAuthenticated => {
                    eprintln!("  Run `colab-cli auth login` to sign in.");
                }
                ColabError::TooManyAssignments => {
                    eprintln!("  Run `colab-cli session stop --name NAME` to remove one.");
                }
                _ => {}
            }
        }

        std::process::exit(1);
    }
}

fn print_error_json(e: &ColabError, verbose: bool) {
    let error = match e {
        ColabError::Drive {
            kind,
            message,
            next_action,
            raw,
        } => {
            let mut value = serde_json::json!({
                "kind": kind,
                "message": message,
                "next_action": next_action,
            });
            if verbose && let Some(raw) = raw {
                value["raw"] = serde_json::Value::String(raw.clone());
            }
            value
        }
        _ => serde_json::json!({
            "kind": error_kind(e),
            "message": e.to_string(),
            "next_action": error_next_action(e),
        }),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": false,
            "error": error,
        }))
        .unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
}

fn error_kind(e: &ColabError) -> &'static str {
    match e {
        ColabError::NotAuthenticated => "not_authenticated",
        ColabError::AuthFailed(_) => "auth_failed",
        ColabError::TokenRefreshFailed { .. } => "token_refresh_failed",
        ColabError::ServerNotFound { .. } => "server_not_found",
        ColabError::TooManyAssignments => "too_many_assignments",
        ColabError::InsufficientQuota => "insufficient_quota",
        ColabError::AccountDenylisted => "account_denylisted",
        ColabError::ApiError { .. } => "api_error",
        ColabError::ParseError(_) => "parse_error",
        ColabError::Config(_) => "config_error",
        ColabError::Drive { .. } => "drive_error",
        ColabError::Io(_) => "io_error",
        ColabError::Network(_) => "network_error",
        ColabError::Json(_) => "json_error",
        ColabError::TomlDe(_) | ColabError::TomlSer(_) => "toml_error",
        ColabError::OAuth(_) => "oauth_error",
    }
}

fn error_next_action(e: &ColabError) -> Option<&'static str> {
    match e {
        ColabError::NotAuthenticated => Some("colab-cli auth login"),
        ColabError::TooManyAssignments => Some("colab-cli session stop --name NAME"),
        _ => None,
    }
}

async fn run(cli: Cli, ui: Ui) -> Result<()> {
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        clap_complete::generate(*shell, &mut cmd, "colab-cli", &mut std::io::stdout());
        return Ok(());
    }

    let json = cli.json;
    match cli.command {
        Commands::Auth { command } => handle_auth(command, ui, json).await,
        Commands::Session { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(command, &config, ui).await
        }
        Commands::Run { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_run_space(command, &config, ui).await
        }
        Commands::Exec { command } => {
            migration(&ui, "colab-cli run ...");
            let config = ColabConfig::load(cli.quiet)?;
            handle_exec(command, &config, ui).await
        }
        Commands::Fs { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_fs(command, &config, ui, json).await
        }
        Commands::Mount { command } => {
            migration(&ui, mount_migration_target(&command));
            let config = ColabConfig::load(cli.quiet)?;
            handle_mount(command, &config, ui, json).await
        }
        Commands::Env { command } => {
            migration(&ui, "colab-cli run install/freeze/restore");
            let config = ColabConfig::load(cli.quiet)?;
            handle_env(command, &config, ui).await
        }
        Commands::Runtime { command } => {
            migration(&ui, runtime_migration_target(&command));
            let config = ColabConfig::load(cli.quiet)?;
            handle_runtime(command, &config, ui, json).await
        }
        Commands::Status { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_status(command, &config, ui, json).await
        }
        Commands::Tools { command } => {
            migration(&ui, "colab-cli settings skills ...");
            handle_tools(command, ui, json)
        }
        Commands::Fleet { command } => handle_fleet(command, ui, json),
        Commands::Slurp { command } => handle_slurp(command, ui, json),
        Commands::Release { command } => handle_release(command, json),
        Commands::Agent { command } => handle_agent(command, ui, json),
        Commands::Continue { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_continue(command, &config, ui, json).await
        }
        Commands::Settings { command } => handle_settings(command, ui, json),
        Commands::Config { command } => {
            migration(&ui, config_migration_target(&command));
            handle_config(command, json)
        }
        Commands::Doctor { .. } => {
            migration(&ui, "colab-cli status check");
            let config = ColabConfig::load(cli.quiet)?;
            handle_status(Some(StatusCommands::Check), &config, ui, json).await
        }
        Commands::BugReport { show_private } => handle_bug_report(show_private, json),
        Commands::Server { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_server(command, &config, ui).await
        }
        Commands::File { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_file(command, &config, ui).await
        }
        Commands::CompatNew(args) => {
            migration(&ui, "colab-cli session new");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::New(args), &config, ui).await
        }
        Commands::CompatSessions => {
            migration(&ui, "colab-cli session list");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::List, &config, ui).await
        }
        Commands::CompatStop(arg) => {
            migration(&ui, "colab-cli session stop");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::Stop(arg), &config, ui).await
        }
        Commands::CompatUpload(args) => {
            migration(&ui, "colab-cli fs push LOCAL REMOTE");
            let config = ColabConfig::load(cli.quiet)?;
            compat_transfer(args, true, &config, ui).await
        }
        Commands::CompatDownload(args) => {
            migration(&ui, "colab-cli fs pull REMOTE LOCAL");
            let config = ColabConfig::load(cli.quiet)?;
            compat_transfer(args, false, &config, ui).await
        }
        Commands::Completions { .. } => unreachable!(),
    }
}

async fn handle_auth(cmd: AuthCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        AuthCommands::Login => {
            let config = ColabConfig::load(ui.quiet)?;
            handle_login(&config, ui).await
        }
        AuthCommands::Logout => {
            auth::logout()?;
            ui.success("Signed out. Credentials cleared.");
            Ok(())
        }
        AuthCommands::Add(args) => handle_auth_add(args, ui),
        AuthCommands::List { show_private } => {
            let store = load_auth_profiles()?;
            let out: Vec<_> = store
                .profiles
                .iter()
                .map(|p| redacted_profile(p, show_private))
                .collect();
            print_value(
                json,
                &serde_json::json!({ "active": store.active, "profiles": out }),
            )
        }
        AuthCommands::Status { name, show_private } => {
            let store = load_auth_profiles()?;
            let profile = store
                .get(&name)
                .ok_or_else(|| ColabError::config(format!("auth profile not found: {name}")))?;
            print_value(json, &redacted_profile(profile, show_private))
        }
        AuthCommands::Use {
            name,
            allow_fallback_account,
        } => {
            let mut store = load_auth_profiles()?;
            let profile = store
                .get(&name)
                .ok_or_else(|| ColabError::config(format!("auth profile not found: {name}")))?
                .clone();
            if allow_fallback_account && !profile.kind.allows_fleet() {
                return Err(ColabError::config(
                    "fallback account rotation is blocked for unknown/free profiles",
                ));
            }
            store.active = Some(name.clone());
            save_auth_profiles(&store)?;
            append_audit(&format!(
                "auth_use profile={name} fallback={allow_fallback_account}"
            ))?;
            ui.success(&format!("using auth profile: {name}"));
            if allow_fallback_account {
                ui.warn("fallback is only for legitimate paid, enterprise, marketplace, or local profiles; it will not dodge limits");
            }
            Ok(())
        }
        AuthCommands::Remove { name } => {
            let mut store = load_auth_profiles()?;
            if !store.remove(&name) {
                return Err(ColabError::config(format!(
                    "auth profile not found: {name}"
                )));
            }
            save_auth_profiles(&store)?;
            ui.success(&format!("removed auth profile: {name}"));
            Ok(())
        }
        AuthCommands::Doctor => {
            let store = load_auth_profiles()?;
            let data = serde_json::json!({
                "profiles": store.profiles.len(),
                "secure_persistent_storage": false,
                "persistent_login": "refused unless keyring or encrypted-file storage is configured",
                "session_only_login": true,
                "shared_credential_cache": false
            });
            print_value(json, &data)
        }
        AuthCommands::ExportRedacted { show_private } => {
            let store = load_auth_profiles()?;
            let mut value = serde_json::to_value(&store)?;
            if !show_private
                && let Some(profiles) = value.get_mut("profiles").and_then(|v| v.as_array_mut())
            {
                for profile in profiles {
                    if let Some(hint) = profile
                        .get_mut("account_hint")
                        .and_then(|v| v.as_str().map(str::to_string))
                    {
                        profile["account_hint"] = serde_json::Value::String(
                            crate::cocli::auth::profiles::redacted_email(&hint, false),
                        );
                    }
                }
            }
            let redacted = crate::cocli::auth::profiles::redact_sensitive(
                &serde_json::to_string_pretty(&value)?,
            );
            println!("{redacted}");
            Ok(())
        }
        AuthCommands::Limits { name } => {
            let store = load_auth_profiles()?;
            let profile = store
                .get(&name)
                .ok_or_else(|| ColabError::config(format!("auth profile not found: {name}")))?;
            let data = serde_json::json!({
                "name": profile.name,
                "kind": profile.kind.to_string(),
                "auto_fallback": false,
                "note": "colab-cli never switches accounts automatically to work around limits"
            });
            print_value(json, &data)
        }
    }
}

fn handle_auth_add(args: AuthProfileArgs, ui: Ui) -> Result<()> {
    let kind: crate::cocli::auth::profiles::AccountKind = args.kind.parse()?;
    let backend = if args.session_only {
        crate::cocli::auth::profiles::StorageBackend::Session
    } else {
        ui.warn("secure keyring support is not compiled in; persistent login refused, creating a session-only profile");
        crate::cocli::auth::profiles::StorageBackend::Session
    };
    let mut store = load_auth_profiles()?;
    store
        .add(crate::cocli::auth::profiles::AuthProfile {
            name: args.name.clone(),
            account_hint: args.account_hint,
            kind,
            created_at: now_rfc3339ish(),
            last_used_at: None,
            storage_backend: backend,
        })
        .map_err(|e| ColabError::config(e.to_string()))?;
    save_auth_profiles(&store)?;
    ui.success(&format!("auth profile added: {}", args.name));
    Ok(())
}

async fn handle_session(cmd: SessionCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        SessionCommands::New(args) => {
            let (variant, accelerator) = session_accelerator(&args)?;
            handle_assign(
                config,
                ui,
                Some(variant),
                accelerator,
                args.name,
                shape_from(args.high_ram),
                args.keepalive,
            )
            .await
        }
        SessionCommands::List => handle_ls(config, ui).await,
        SessionCommands::Status(SessionNameArg { session }) => {
            migration(&ui, "colab-cli status session --name NAME");
            handle_info(config, ui, session).await
        }
        SessionCommands::Stop(SessionNameArg { session }) => handle_rm(config, ui, session).await,
        SessionCommands::Url { session, open } => handle_url(config, ui, session, open).await,
        SessionCommands::Last => {
            let manager = make_manager(config)?;
            let servers = manager.list_local()?;
            let last = servers
                .iter()
                .max_by_key(|s| s.date_assigned)
                .ok_or_else(|| {
                    ColabError::config("no active session - run `colab-cli session list`")
                })?;
            ui.print_server_status(last);
            Ok(())
        }
    }
}

async fn handle_run_space(cmd: RunCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        RunCommands::Script {
            script,
            session,
            args,
        } => {
            handle_exec(
                ExecCommands::Run {
                    script,
                    session,
                    args,
                },
                config,
                ui,
            )
            .await
        }
        RunCommands::Py { session, code } => {
            handle_exec(ExecCommands::Py { session, code }, config, ui).await
        }
        RunCommands::Notebook {
            notebook,
            session,
            out,
        } => {
            handle_exec(
                ExecCommands::Nb {
                    notebook,
                    session,
                    out,
                },
                config,
                ui,
            )
            .await
        }
        RunCommands::Repl { session } => {
            handle_exec(ExecCommands::Repl { session }, config, ui).await
        }
        RunCommands::Shell { session } => {
            handle_exec(ExecCommands::Shell { session }, config, ui).await
        }
        RunCommands::Install {
            packages,
            requirements,
            session,
        } => {
            if let Some(requirements) = requirements {
                if !packages.is_empty() {
                    return Err(ColabError::config(
                        "run install accepts packages or -r requirements.txt, not both",
                    ));
                }
                handle_env(
                    EnvCommands::Restore {
                        requirements,
                        session,
                    },
                    config,
                    ui,
                )
                .await
            } else {
                handle_env(EnvCommands::Install { packages, session }, config, ui).await
            }
        }
        RunCommands::Freeze { session } => {
            handle_env(EnvCommands::Freeze { session }, config, ui).await
        }
        RunCommands::Restore {
            requirements,
            session,
        } => {
            handle_env(
                EnvCommands::Restore {
                    requirements,
                    session,
                },
                config,
                ui,
            )
            .await
        }
        RunCommands::Last { confirm } => {
            handle_exec(ExecCommands::Last { confirm }, config, ui).await
        }
        RunCommands::History => Err(ColabError::config(
            "run history has no command store yet - rerun commands explicitly",
        )),
    }
}

async fn handle_exec(cmd: ExecCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        ExecCommands::Run {
            script,
            session,
            args,
        } => {
            let mut command = vec!["python".to_string(), script];
            command.extend(args);
            handle_run(config, ui, session, command).await
        }
        ExecCommands::Py { session, code } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-c".into(), code],
            )
            .await
        }
        ExecCommands::Nb {
            notebook,
            session,
            out,
        } => {
            let mut command = vec![
                "python".into(),
                "-m".into(),
                "jupyter".into(),
                "nbconvert".into(),
                "--to".into(),
                "notebook".into(),
                "--execute".into(),
                notebook,
            ];
            if let Some(out) = out {
                command.extend(["--output".into(), out]);
            }
            handle_run(config, ui, session, command).await
        }
        ExecCommands::Repl { session } => {
            handle_run(config, ui, session, vec!["python".into()]).await
        }
        ExecCommands::Shell { session } => handle_shell(config, ui, session).await,
        ExecCommands::Last { confirm } => {
            if !confirm {
                return Err(ColabError::config("exec last requires --confirm"));
            }
            Err(ColabError::config(
                "no last command store yet - rerun the command explicitly",
            ))
        }
    }
}

async fn handle_fs(cmd: FsCommands, config: &ColabConfig, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        FsCommands::Ls { path, session } => {
            let args = vec![
                "-lah".to_string(),
                path.unwrap_or_else(|| "/content".into()),
            ];
            handle_file_ls(config, ui, session, args).await
        }
        FsCommands::Push { src, dest, session } => {
            handle_upload(config, ui, session, &src, Some(&dest)).await
        }
        FsCommands::Pull { src, dest, session } => {
            handle_download(config, ui, session, &src, dest.as_deref()).await
        }
        FsCommands::Rm {
            path,
            session,
            recursive,
            yes,
        } => {
            if !yes {
                return Err(ColabError::config(
                    "refusing remote rm without --yes; destructive commands need explicit confirmation",
                ));
            }
            let mut args = Vec::new();
            if recursive {
                args.push("-r".to_string());
            }
            args.push(path);
            handle_file_rm(config, ui, session, args).await
        }
        FsCommands::Edit { path, session } => handle_file_edit(config, ui, session, path).await,
        FsCommands::Sync(args) => handle_fs_sync(args, ui, json),
        FsCommands::Diff(args) => handle_fs_diff(args, ui),
        FsCommands::Changed(args) => handle_fs_diff(args, ui),
        FsCommands::Drive { command } => handle_fs_drive(command, config, ui, json).await,
    }
}

async fn handle_fs_drive(
    cmd: FsDriveCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match cmd {
        FsDriveCommands::Mount {
            session,
            path,
            dry_run,
            timeout,
            open,
        } => {
            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "action": "drive.mount",
                        "path": path,
                        "needs_session": true,
                        "needs_kernel": true,
                        "would_execute": true
                    }),
                );
            }
            drive_mount(config, ui, json, session, path, timeout, open).await
        }
        FsDriveCommands::Status { session, dry_run } => {
            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "action": "drive.status",
                        "needs_session": true,
                        "next_action": "run `colab-cli fs drive mount --session NAME` if not mounted"
                    }),
                );
            }
            let status = drive_status(config, ui, session, DEFAULT_DRIVE_PATH).await?;
            print_drive_status(&status, json, ui)
        }
        FsDriveCommands::List { session } => {
            handle_file_ls(
                config,
                ui,
                session,
                vec!["-lah".to_string(), DEFAULT_DRIVE_PATH.to_string()],
            )
            .await
        }
        FsDriveCommands::Unmount { session, dry_run } => {
            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "action": "drive.unmount",
                        "needs_session": true,
                        "would_execute": true
                    }),
                );
            }
            drive_unmount(config, ui, json, session).await
        }
        FsDriveCommands::Path { .. } => {
            println!("{DEFAULT_DRIVE_PATH}");
            Ok(())
        }
    }
}

async fn handle_mount(cmd: MountCommands, config: &ColabConfig, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        MountCommands::Drive {
            session,
            path,
            timeout,
            open,
            dry_run,
        } => {
            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "action": "drive.mount",
                        "path": path,
                        "needs_session": true,
                        "needs_kernel": true,
                        "would_execute": true
                    }),
                );
            }
            drive_mount(config, ui, json, session, path, timeout, open).await
        }
        MountCommands::List { session } => {
            let status = drive_status(config, ui, session, DEFAULT_DRIVE_PATH).await?;
            print_drive_status(&status, json, ui)
        }
    }
}

const DEFAULT_DRIVE_PATH: &str = "/content/drive";

#[derive(Debug, Clone, serde::Serialize)]
struct DriveStatus {
    ok: bool,
    mounted: Option<bool>,
    path: String,
    next_action: Option<String>,
}

async fn drive_mount(
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    session: Option<String>,
    path: String,
    timeout_secs: u64,
    open: bool,
) -> Result<()> {
    let status = drive_status(config, ui, session.clone(), &path).await?;
    if status.mounted == Some(true) {
        return print_drive_mount_success(json, &path, "Drive already mounted");
    }

    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;

    if open {
        let url = crate::cocli::runtime::session_url(&config.colab_domain, &server.endpoint)
            .map_err(|e| ColabError::config(e.to_string()))?;
        open_url(&url)?;
        ui.info("opened browser");
    }

    let timeout = std::time::Duration::from_secs(timeout_secs.max(1));
    preflight_drive_kernel(manager.client(), &server, timeout).await?;

    let output =
        runner::execute_colab_cell(manager.client(), &server, &drive_mount_cell(&path), timeout)
            .await?;
    drive_output_to_result(&output)?;

    if output.timed_out {
        return Err(drive_approval_required(Some(output.raw_text())));
    }

    let after = drive_status(config, ui, session, &path).await?;
    if after.mounted == Some(true) || drive_mount_output_looks_ok(&output.raw_text()) {
        print_drive_mount_success(json, &path, "Drive mounted")
    } else {
        Err(ColabError::drive(
            "drive_status_unknown",
            "Could not confirm Drive status after mount",
            Some("colab-cli fs drive status"),
            Some(output.raw_text()),
        ))
    }
}

async fn drive_unmount(
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    session: Option<String>,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    preflight_drive_kernel(
        manager.client(),
        &server,
        std::time::Duration::from_secs(30),
    )
    .await?;

    let output = runner::execute_colab_cell(
        manager.client(),
        &server,
        "from google.colab import drive\ndrive.flush_and_unmount()",
        std::time::Duration::from_secs(60),
    )
    .await?;
    drive_output_to_result(&output)?;
    if output.timed_out {
        return Err(ColabError::drive(
            "drive_unmount_timeout",
            "Drive unmount did not finish before timeout",
            Some("colab-cli fs drive status"),
            Some(output.raw_text()),
        ));
    }
    if json {
        print_value(
            true,
            &serde_json::json!({
                "ok": true,
                "mounted": false,
                "path": DEFAULT_DRIVE_PATH,
                "next_action": null
            }),
        )
    } else {
        ui.success("Drive unmounted");
        Ok(())
    }
}

async fn drive_status(
    config: &ColabConfig,
    ui: Ui,
    session: Option<String>,
    path: &str,
) -> Result<DriveStatus> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let out = match runner::capture_remote_command(
        manager.client(),
        &server,
        &drive_status_probe_command(path),
    )
    .await
    {
        Ok(out) => out,
        Err(_) => {
            return Ok(DriveStatus {
                ok: false,
                mounted: None,
                path: path.to_string(),
                next_action: Some("colab-cli status check".to_string()),
            });
        }
    };
    Ok(parse_drive_status(&out, path))
}

async fn preflight_drive_kernel(
    client: &ColabClient,
    server: &StoredServer,
    timeout: std::time::Duration,
) -> Result<()> {
    let output =
        runner::execute_colab_cell(client, server, drive_preflight_cell(), timeout).await?;
    drive_output_to_result(&output)?;
    if output.timed_out {
        return Err(ColabError::drive(
            "drive_kernel_timeout",
            "Drive mount needs a responsive Colab kernel session",
            Some("colab-cli session url --open"),
            Some(output.raw_text()),
        ));
    }
    if output.stdout.trim() == "true" {
        Ok(())
    } else {
        Err(ColabError::drive(
            "drive_kernel_context_required",
            "Drive mount needs a Colab kernel session, not a plain Python process",
            Some("colab-cli session url --open"),
            Some(output.raw_text()),
        ))
    }
}

fn print_drive_status(status: &DriveStatus, json: bool, ui: Ui) -> Result<()> {
    if json {
        return print_value(true, status);
    }
    match status.mounted {
        Some(true) => ui.success(&format!("Drive mounted at {}", status.path)),
        Some(false) => {
            println!("Drive is not mounted");
            if let Some(next) = &status.next_action {
                println!("next: {next}");
            }
        }
        None => {
            println!("Could not confirm Drive status");
            if let Some(next) = &status.next_action {
                println!("next: {next}");
            }
        }
    }
    Ok(())
}

fn print_drive_mount_success(json: bool, path: &str, msg: &str) -> Result<()> {
    if json {
        print_value(
            true,
            &serde_json::json!({
                "ok": true,
                "mounted": true,
                "path": path,
                "next_action": null
            }),
        )
    } else {
        println!("\u{2713} {msg} at {path}");
        Ok(())
    }
}

fn drive_status_probe_command(path: &str) -> String {
    let path = shell_single_quote(path);
    format!(
        "drive_path={path}; \
         if [ -d \"$drive_path/MyDrive\" ] || [ -d \"$drive_path/My Drive\" ]; then \
           echo mounted; \
         elif [ -d \"$drive_path\" ] && find \"$drive_path\" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null | grep -q .; then \
           echo mounted; \
         elif [ -d \"$drive_path\" ]; then \
           echo not_mounted; \
         else \
           echo not_mounted; \
         fi; \
         printf 'path=%s\\n' \"$drive_path\"; \
         (mount | grep -Ei 'drive|fuse' | head -n 3) >/dev/null 2>&1 || true"
    )
}

fn parse_drive_status(output: &str, path: &str) -> DriveStatus {
    let first = output.lines().next().map(str::trim);
    match first {
        Some("mounted") => DriveStatus {
            ok: true,
            mounted: Some(true),
            path: path.to_string(),
            next_action: None,
        },
        Some("not_mounted") => DriveStatus {
            ok: true,
            mounted: Some(false),
            path: path.to_string(),
            next_action: Some("colab-cli fs drive mount".to_string()),
        },
        _ => DriveStatus {
            ok: false,
            mounted: None,
            path: path.to_string(),
            next_action: Some("colab-cli status check".to_string()),
        },
    }
}

fn drive_preflight_cell() -> &'static str {
    "import IPython\nip = IPython.get_ipython()\nprint('true' if getattr(ip, 'kernel', None) is not None else 'false')"
}

fn drive_mount_cell(path: &str) -> String {
    let path = match serde_json::to_string(path) {
        Ok(path) => path,
        Err(_) => "\"/content/drive\"".to_string(),
    };
    format!("from google.colab import drive\ndrive.mount({path}, force_remount=False)")
}

fn drive_output_to_result(output: &runner::CellOutput) -> Result<()> {
    let raw = output.raw_text();
    if let Some(err) = classify_drive_error(&raw) {
        return Err(err);
    }
    if output.error_name.is_some() {
        return Err(ColabError::drive(
            "drive_mount_failed",
            "Drive command failed",
            Some("colab-cli fs drive status"),
            Some(raw),
        ));
    }
    Ok(())
}

fn classify_drive_error(raw: &str) -> Option<ColabError> {
    let lower = raw.to_ascii_lowercase();
    if raw.contains("AttributeError: 'NoneType' object has no attribute 'kernel'")
        || lower.contains("get_ipython")
    {
        return Some(ColabError::drive(
            "drive_kernel_context_required",
            "Drive mount needs a Colab kernel session, not a plain Python process",
            Some("colab-cli session url --open"),
            Some(raw.to_string()),
        ));
    }
    if lower.contains("google.colab._message")
        || lower.contains("blocking_request")
        || lower.contains("request_auth")
        || lower.contains("kernel requested input")
    {
        return Some(drive_approval_required(Some(raw.to_string())));
    }
    if lower.contains("mounting drive is unsupported")
        || lower.contains("drive.mount is not supported")
        || lower.contains("colab enterprise")
    {
        return Some(ColabError::drive(
            "drive_unsupported",
            "Drive mount is not supported for this runtime",
            Some("colab-cli status check"),
            Some(raw.to_string()),
        ));
    }
    None
}

fn drive_approval_required(raw: Option<String>) -> ColabError {
    ColabError::drive(
        "drive_browser_approval_required",
        "Drive needs browser approval",
        Some("colab-cli session url --open"),
        raw,
    )
}

fn drive_mount_output_looks_ok(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("mounted at") || lower.contains("already mounted")
}

fn shell_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

async fn handle_env(cmd: EnvCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        EnvCommands::Install { packages, session } => {
            if packages.is_empty() {
                return Err(ColabError::config(
                    "run install needs packages or -r requirements.txt",
                ));
            }
            handle_run(
                config,
                ui,
                session,
                crate::cocli::runtime::pip_install_command(&packages),
            )
            .await
        }
        EnvCommands::Freeze { session } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-m".into(), "pip".into(), "freeze".into()],
            )
            .await
        }
        EnvCommands::Restore {
            requirements,
            session,
        } => {
            handle_run(
                config,
                ui,
                session,
                vec![
                    "python".into(),
                    "-m".into(),
                    "pip".into(),
                    "install".into(),
                    "-r".into(),
                    requirements,
                ],
            )
            .await
        }
    }
}

async fn handle_runtime(
    cmd: RuntimeCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match cmd {
        RuntimeCommands::Info { backend } => {
            if backend {
                print_backend_info(json)
            } else {
                handle_info(config, ui, None).await
            }
        }
        RuntimeCommands::BackendInfo | RuntimeCommands::Versions => print_backend_info(json),
        RuntimeCommands::Gpu => {
            ui.info("GPU details require a session; use `colab-cli run py --code \"import torch; print(torch.cuda.get_device_name(0))\"`.");
            Ok(())
        }
        RuntimeCommands::Tpu => {
            ui.info(
                "TPU details require a session; use `colab-cli status runtime --backend` for package baselines.",
            );
            Ok(())
        }
        RuntimeCommands::Fit { model } => {
            let verdict = runtime_fit(&model);
            print_value(json, &serde_json::json!({ "model": model, "fit": verdict }))
        }
    }
}

fn print_backend_info(json: bool) -> Result<()> {
    let data = serde_json::json!({
        "apt": crate::cocli::runtime::backend_info_url("apt-list.txt"),
        "pip": crate::cocli::runtime::backend_info_url("pip-freeze.txt"),
        "note": "backend-info can lag production runtimes by one or two days"
    });
    print_value(json, &data)
}

fn runtime_fit(model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("70b") || m.contains("405b") {
        "nope"
    } else if m.contains("13b") || m.contains("34b") {
        "tight"
    } else if m.contains("7b") || m.contains("8b") || m.contains("small") {
        "probably-fits"
    } else {
        "unknown"
    }
}

async fn handle_status(
    cmd: Option<StatusCommands>,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match cmd {
        None => {
            let auth_state = auth::current_account()?.map(|a| a.email);
            print_value(
                json,
                &serde_json::json!({
                    "session": "run `colab-cli session list`",
                    "auth": auth_state.is_some(),
                    "runtime": "run `colab-cli status runtime --all`",
                    "files": "run `colab-cli fs changed LOCAL REMOTE`",
                    "drive": "run `colab-cli fs drive status --session NAME`",
                    "next_action": if auth_state.is_some() { "run `colab-cli session list`" } else { "run `colab-cli auth login`" }
                }),
            )
        }
        Some(StatusCommands::Session { name }) => handle_info(config, ui, name).await,
        Some(StatusCommands::Runtime {
            backend,
            gpu,
            tpu,
            versions,
            all,
            fit,
        }) => {
            if let Some(model) = fit {
                let verdict = runtime_fit(&model);
                return print_value(json, &serde_json::json!({ "model": model, "fit": verdict }));
            }
            if backend || versions || all {
                print_backend_info(json)?;
            }
            if gpu || all {
                handle_runtime(RuntimeCommands::Gpu, config, ui, json).await?;
            }
            if tpu || all {
                handle_runtime(RuntimeCommands::Tpu, config, ui, json).await?;
            }
            if !(backend || gpu || tpu || versions || all) {
                handle_runtime(RuntimeCommands::Info { backend: false }, config, ui, json).await?;
            }
            Ok(())
        }
        Some(StatusCommands::Auth) => {
            let auth_state = auth::current_account()?.map(|a| a.email);
            print_value(
                json,
                &serde_json::json!({
                    "signed_in": auth_state.is_some(),
                    "email": auth_state.as_ref().map(|email| crate::cocli::auth::redaction::redacted_email(email, false)),
                    "next_action": if auth_state.is_some() { "run `colab-cli session list`" } else { "run `colab-cli auth login`" }
                }),
            )
        }
        Some(StatusCommands::Fs) => print_value(
            json,
            &serde_json::json!({
                "sync": "manifest dry-run available",
                "next_action": "run `colab-cli fs changed LOCAL REMOTE`"
            }),
        ),
        Some(StatusCommands::Drive) => print_value(
            json,
            &serde_json::json!({
                "status": "needs live session",
                "next_action": "run `colab-cli fs drive status --session NAME`"
            }),
        ),
        Some(StatusCommands::Slurp { config }) => print_value(
            json,
            &serde_json::json!({
                "config": config,
                "exists": Path::new(&config).exists(),
                "next_action": if Path::new(&config).exists() { "run `colab-cli slurp explain`" } else { "run `colab-cli slurp init`" }
            }),
        ),
        Some(StatusCommands::Fleet { config }) => {
            if Path::new(&config).exists() {
                handle_fleet(
                    FleetCommands::Plan(FleetConfigArgs {
                        config,
                        dry_run: true,
                        cost: true,
                        allow_fallback_account: false,
                    }),
                    ui,
                    json,
                )
            } else {
                print_value(
                    json,
                    &serde_json::json!({
                        "config": config,
                        "exists": false,
                        "next_action": "run `colab-cli slurp init`"
                    }),
                )
            }
        }
        Some(StatusCommands::Quick) => {
            let auth_state = auth::current_account()?.is_some();
            print_value(
                json,
                &serde_json::json!({
                    "auth": auth_state,
                    "config": config::config_path().ok().map(|p| p.exists()).unwrap_or(false),
                    "data_dir": config::data_dir().ok().map(|p| p.display().to_string()),
                    "next_action": if auth_state { "run `colab-cli session list`" } else { "run `colab-cli auth login`" }
                }),
            )
        }
        Some(StatusCommands::Check) => {
            let auth_state = auth::current_account()?.map(|a| a.email);
            print_value(
                json,
                &serde_json::json!({
                    "auth": auth_state.is_some(),
                    "config_path": config::config_path().ok().map(|p| p.display().to_string()),
                    "unsafe_code": "forbidden by package lints",
                    "next_action": if auth_state.is_some() { "run `colab-cli status quick`" } else { "run `colab-cli auth login`" }
                }),
            )
        }
        Some(StatusCommands::Run) => print_value(
            json,
            &serde_json::json!({
                "note": "runtime setup checks require a live session",
                "next_action": "run `colab-cli run py --session NAME --code \"import sys; print(sys.version)\"`"
            }),
        ),
        Some(StatusCommands::Paths) => print_value(
            json,
            &serde_json::json!({
                "config_dir": config::config_dir().ok().map(|p| p.display().to_string()),
                "data_dir": config::data_dir().ok().map(|p| p.display().to_string()),
                "config_path": config::config_path().ok().map(|p| p.display().to_string())
            }),
        ),
    }
}

fn handle_tools(cmd: ToolsCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        ToolsCommands::List { json: local_json } => handle_skills(
            SkillCommands::List {
                json: local_json,
                category: None,
                risk: None,
                needs_session: false,
            },
            ui,
            json,
        ),
        ToolsCommands::Inspect {
            tool_name,
            json: local_json,
        } => handle_skills(
            SkillCommands::Inspect {
                name: tool_name,
                json: local_json,
            },
            ui,
            json,
        ),
        ToolsCommands::Run {
            tool_name,
            input_json,
            yes,
        } => handle_skills(
            SkillCommands::Run {
                name: tool_name,
                input_json,
                yes,
            },
            ui,
            json,
        ),
    }
}

fn handle_settings(cmd: SettingsCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        SettingsCommands::Get => handle_config(ConfigCommands::Get, json),
        SettingsCommands::Set { key, value } => {
            handle_config(ConfigCommands::Set { key, value }, json)
        }
        SettingsCommands::Path => handle_config(ConfigCommands::Path, json),
        SettingsCommands::Edit => handle_config(ConfigCommands::Open, json),
        SettingsCommands::Reset { yes } => {
            if !yes {
                return Err(ColabError::config(
                    "settings reset needs --yes; it rewrites the local UI config",
                ));
            }
            let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
            config::CocliConfig::default()
                .save(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            ui.success("settings reset");
            Ok(())
        }
        SettingsCommands::Skills { command } => handle_skills(command, ui, json),
    }
}

fn handle_skills(cmd: SkillCommands, _ui: Ui, json: bool) -> Result<()> {
    match cmd {
        SkillCommands::List {
            json: local_json,
            category,
            risk,
            needs_session,
        } => {
            let rows = skill_rows(category.as_deref(), risk.as_deref(), needs_session);
            if json || local_json {
                print_value(true, &rows)
            } else {
                println!(
                    "{:<18} {:<8} {:<13} {:<7} {:<7} Summary",
                    "Skill", "Risk", "Needs session", "Network", "Dry-run"
                );
                for row in rows {
                    println!(
                        "{:<18} {:<8} {:<13} {:<7} {:<7} {}",
                        row.name,
                        row.risk,
                        yes_no(row.needs_session),
                        yes_no(row.network),
                        yes_no(row.dry_run),
                        row.summary
                    );
                }
                Ok(())
            }
        }
        SkillCommands::Inspect {
            name,
            json: local_json,
        } => {
            let row = skill_rows(None, None, false)
                .into_iter()
                .find(|row| row.name == name)
                .ok_or_else(|| ColabError::config(format!("unknown skill: {name}")))?;
            if json || local_json {
                print_value(true, &row)
            } else {
                println!("skill\t{}", row.name);
                println!("category\t{}", row.category);
                println!("risk\t{}", row.risk);
                println!("needs_session\t{}", row.needs_session);
                println!("network\t{}", row.network);
                println!("dry_run\t{}", row.dry_run);
                println!("summary\t{}", row.summary);
                println!(
                    "example\tcolab-cli settings skills run {} --json '{{}}'",
                    row.name
                );
                Ok(())
            }
        }
        SkillCommands::Run {
            name,
            input_json,
            yes,
        } => {
            let input: serde_json::Value = serde_json::from_str(&input_json)?;
            let output = crate::cocli::tools::ToolRegistry::run_plan(&name, input, yes)
                .map_err(|e| ColabError::config(e.to_string()))?;
            print_value(true, &output)
        }
        SkillCommands::Enable { name } | SkillCommands::Disable { name } => {
            Err(ColabError::config(format!(
                "skill toggles are not implemented; built-in skill `{name}` is always available"
            )))
        }
    }
}

#[derive(serde::Serialize)]
struct SkillRow {
    name: String,
    category: String,
    risk: &'static str,
    needs_session: bool,
    network: bool,
    dry_run: bool,
    summary: String,
}

fn skill_rows(category: Option<&str>, risk: Option<&str>, needs_session: bool) -> Vec<SkillRow> {
    crate::cocli::tools::ToolRegistry::specs()
        .into_iter()
        .map(|spec| {
            let category = spec.name.split('.').next().unwrap_or("misc").to_string();
            SkillRow {
                name: spec.name,
                category,
                risk: display_risk(spec.risk),
                needs_session: spec.requires_session,
                network: spec.requires_network,
                dry_run: spec.dry_run,
                summary: sentence_case(&spec.description),
            }
        })
        .filter(|row| category.is_none_or(|want| row.category == want))
        .filter(|row| risk.is_none_or(|want| row.risk == want))
        .filter(|row| !needs_session || row.needs_session)
        .collect()
}

fn display_risk(risk: crate::cocli::r#continue::manifest::RiskLevel) -> &'static str {
    match risk {
        crate::cocli::r#continue::manifest::RiskLevel::Low => "low",
        crate::cocli::r#continue::manifest::RiskLevel::Network => "medium",
        crate::cocli::r#continue::manifest::RiskLevel::Destructive => "high",
    }
}

fn sentence_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn handle_fleet(cmd: FleetCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        FleetCommands::Plan(args) => {
            let (cfg, plan, findings) = fleet_plan_from_args(&args)?;
            print_fleet_plan(&cfg, &plan, &findings, json || args.cost)
        }
        FleetCommands::Start(args) | FleetCommands::Exec(args) => {
            let (cfg, plan, findings) = fleet_plan_from_args(&args)?;
            refuse_if_needed(&findings)?;
            if args.dry_run {
                return print_fleet_plan(&cfg, &plan, &findings, json || args.cost);
            }
            Err(ColabError::config(
                "fleet execution is deferred; run `colab-cli fleet plan --cost`",
            ))
        }
        FleetCommands::Doctor => {
            migration(&ui, "colab-cli status fleet");
            let data = serde_json::json!({
                "fleet_mode": "compliant",
                "free_tier_cluster_backend": false,
                "fallback_rotation": false,
                "next_action": "run `colab-cli fleet plan --config slurp.toml`"
            });
            print_value(json, &data)
        }
    }
}

fn handle_slurp(cmd: SlurpCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        SlurpCommands::Init { out } => {
            if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                print!("Slurp name [llama-batch-run]: ");
                std::io::stdout().flush()?;
                let mut name = String::new();
                std::io::stdin().read_line(&mut name)?;
                let name = name.trim();
                let name = if name.is_empty() {
                    "llama-batch-run"
                } else {
                    name
                };
                std::fs::write(
                    &out,
                    crate::cocli::slurp::config::SlurpConfig::sample()
                        .replace("llama-batch-run", name),
                )?;
            } else {
                std::fs::write(&out, crate::cocli::slurp::config::SlurpConfig::sample())?;
            }
            if !ui.quiet {
                println!("slurp ▸ tiny plan, big snack");
            }
            ui.success(&format!("wrote {out}"));
            Ok(())
        }
        SlurpCommands::Check(args) => {
            let cfg = load_slurp(&args.config)?;
            let findings = crate::cocli::fleet::compliance::validate_slurp(&cfg);
            print_value(json, &findings)
        }
        SlurpCommands::Plan(args) => handle_fleet(FleetCommands::Plan(args), ui, json),
        SlurpCommands::Run(args) | SlurpCommands::Resume(args) => {
            handle_fleet(FleetCommands::Exec(args), ui, json)
        }
        SlurpCommands::Explain(args) => {
            let cfg = load_slurp(&args.config)?;
            if json {
                print_value(true, &serde_json::json!({ "explain": cfg.explain() }))
            } else {
                println!("{}", cfg.explain());
                Ok(())
            }
        }
        SlurpCommands::Doctor(args) => {
            migration(&ui, "colab-cli status slurp");
            let cfg = load_slurp(&args.config)?;
            let mut findings = crate::cocli::fleet::compliance::validate_slurp(&cfg);
            if !Path::new(&cfg.work.entry).exists() {
                findings.push(crate::cocli::fleet::compliance::ComplianceFinding {
                    level: crate::cocli::fleet::compliance::ComplianceLevel::Warn,
                    message: format!("entry file not found locally: {}", cfg.work.entry),
                    next_action: "check work.entry or run from the project root".into(),
                });
            }
            print_value(json, &findings)
        }
        SlurpCommands::Schema => {
            let schema = serde_json::json!({
                "required": ["slurp", "budget", "accounts", "work"],
                "forbidden": ["tokens", "passwords", "hidden_commands"],
                "seed": ["secure", "integer"],
                "mode": ["compliant"]
            });
            print_value(true, &schema)
        }
    }
}

fn handle_release(cmd: ReleaseCommands, json: bool) -> Result<()> {
    match cmd {
        ReleaseCommands::Name { version } => {
            let name = crate::cocli::release::names::release_name(
                &version,
                std::env::var("RELEASE_NAME").ok().as_deref(),
            );
            println!("{name}");
            Ok(())
        }
        ReleaseCommands::Notes { version, commits } => {
            let refs: Vec<_> = commits.iter().map(String::as_str).collect();
            let notes = crate::cocli::release::names::release_notes(&version, &refs);
            print_value(json, &notes)
        }
        ReleaseCommands::Bump { commits, pre_1 } => {
            let refs: Vec<_> = commits.iter().map(String::as_str).collect();
            println!(
                "{}",
                crate::cocli::release::names::semver_bump(&refs, pre_1)
            );
            Ok(())
        }
    }
}

fn handle_agent(cmd: AgentCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        AgentCommands::Tools => handle_tools(ToolsCommands::List { json }, ui, json),
        AgentCommands::Plan { goal, out } => {
            let plan = format!(
                "goal = {goal:?}\nconfirm_required = true\n\n[[steps]]\ntool = \"status.check\"\ninput = {{}}\n"
            );
            if let Some(out) = out {
                std::fs::write(&out, plan)?;
                ui.success(&format!("plan written: {out}"));
            } else {
                print!("{plan}");
            }
            Ok(())
        }
        AgentCommands::Run { plan, confirm } => {
            if !confirm {
                return Err(ColabError::config(
                    "agent run requires --confirm; plans never execute implicitly",
                ));
            }
            let body = std::fs::read_to_string(&plan)?;
            append_audit(&format!("agent_run plan={plan} bytes={}", body.len()))?;
            ui.success("agent plan accepted for confirmed execution audit");
            ui.info(
                "execution hooks are intentionally limited to built-in tool plans in this release",
            );
            Ok(())
        }
        AgentCommands::AuditPlan { plan } => {
            let body = std::fs::read_to_string(&plan)?;
            let data = serde_json::json!({
                "plan": plan,
                "bytes": body.len(),
                "confirm_required": !body.contains("confirm_required = false")
            });
            print_value(json, &data)
        }
        AgentCommands::Slurp { goal, out } => {
            if goal.to_ascii_lowercase().contains("bypass")
                || goal.to_ascii_lowercase().contains("keepalive")
            {
                return Err(ColabError::config(
                    "agent Slurp drafts cannot suggest bypassing limits or anti-idle scripts",
                ));
            }
            let plan = crate::cocli::slurp::config::SlurpConfig::sample();
            let body = format!(
                "# compliance: Slurp can plan this, but it will not bypass Colab rules\n# goal: {goal}\n{plan}"
            );
            if let Some(out) = out {
                std::fs::write(&out, body)?;
                ui.success(&format!("Slurp draft written: {out}"));
            } else {
                print!("{body}");
            }
            Ok(())
        }
        AgentCommands::AuditSlurp { config } => {
            let cfg = load_slurp(&config)?;
            print_value(json, &crate::cocli::fleet::compliance::validate_slurp(&cfg))
        }
        AgentCommands::ExplainSlurp { config } => {
            let cfg = load_slurp(&config)?;
            println!("{}", cfg.explain());
            Ok(())
        }
    }
}

async fn handle_continue(
    cmd: ContinueCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match cmd {
        ContinueCommands::Save {
            session,
            name,
            artifacts,
        } => {
            let manager = make_manager(config)?;
            let servers = manager.list_local()?;
            let server = resolve_server(&servers, Some(&session))?;
            let mut manifest = crate::cocli::r#continue::manifest::ContinuationManifest::new(
                chrono::Utc::now().to_rfc3339(),
                &name,
            );
            manifest.session.id = Some(server.id.to_string());
            manifest.session.name = server.label.clone();
            manifest.runtime_class = server.variant.to_string();
            manifest.accelerator_type = server.accelerator.clone();
            manifest.artifacts = artifacts;
            manifest.git = git_snapshot();
            write_continuation(config, &name, &manifest)?;
            if json {
                print_value(true, &manifest)?;
            } else {
                ui.success("checkpoint tucked away");
                println!("cocli ▸ fast path found");
                ui.info("saved metadata and replay plan; live Python variables were not copied");
            }
            Ok(())
        }
        ContinueCommands::Inspect { name } => {
            let manifest = read_continuation(config, &name)?;
            print_value(json, &manifest)
        }
        ContinueCommands::Export { name, out } => {
            let src = continuation_path(config, &name);
            std::fs::copy(src, &out)?;
            ui.success(&format!("exported continuation bundle: {out}"));
            Ok(())
        }
        ContinueCommands::Import { bundle } => {
            let bytes = std::fs::read(&bundle)?;
            let manifest =
                crate::cocli::r#continue::manifest::ContinuationManifest::from_json(&bytes)
                    .map_err(|e| ColabError::parse(e.to_string()))?;
            let name = Path::new(&bundle)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&manifest.session.name);
            write_continuation(config, name, &manifest)?;
            ui.success(&format!("imported continuation: {name}"));
            Ok(())
        }
        ContinueCommands::Clean { older_than } => {
            let days = crate::cocli::util::time::parse_days(&older_than)
                .map_err(|e| ColabError::config(e.to_string()))?;
            let cutoff = std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(days * 24 * 60 * 60))
                .ok_or_else(|| ColabError::config("invalid clean cutoff"))?;
            let mut removed = 0usize;
            let dir = continuations_dir(config);
            if dir.exists() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let meta = entry.metadata()?;
                    if meta.modified().is_ok_and(|t| t < cutoff) {
                        std::fs::remove_file(entry.path())?;
                        removed += 1;
                    }
                }
            }
            ui.success(&format!("removed {removed} old continuation(s)"));
            Ok(())
        }
        ContinueCommands::Resume {
            name,
            new_runtime,
            gpu,
            replay_all,
            dry_run,
        } => {
            let manifest = read_continuation(config, &name)?;
            let mut steps = Vec::new();
            if replay_all {
                steps.extend(manifest.executed_steps.clone());
            }
            steps.extend(manifest.pending_steps.clone());

            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "continuation": name,
                        "would_create_runtime": new_runtime,
                        "would_replay_steps": steps.len(),
                        "process_memory_restored": false
                    }),
                );
            }

            if new_runtime {
                handle_assign(
                    config,
                    ui,
                    Some(if gpu.is_some() {
                        Variant::Gpu
                    } else {
                        Variant::Cpu
                    }),
                    gpu,
                    Some(manifest.session.name.clone()),
                    Shape::Standard,
                    false,
                )
                .await?;
            }

            for step in &steps {
                handle_run(
                    config,
                    ui,
                    Some(manifest.session.name.clone()),
                    step.command.clone(),
                )
                .await?;
            }

            let report = serde_json::json!({
                "continuation": name,
                "replayed_steps": steps.len(),
                "new_runtime": new_runtime,
                "process_memory_restored": false
            });
            let report_path = continuations_dir(config).join(format!("{name}.resume-report.json"));
            if let Some(parent) = report_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
            print_value(json, &report)
        }
        ContinueCommands::Last => {
            let name = newest_continuation(config)?.ok_or_else(|| {
                ColabError::config("resume needs a checkpoint - run `colab-cli continue list`")
            })?;
            let manifest = read_continuation(config, &name)?;
            print_value(json, &manifest)
        }
    }
}

fn handle_config(cmd: ConfigCommands, json: bool) -> Result<()> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    match cmd {
        ConfigCommands::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigCommands::Get => {
            let cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            print_value(json, &cfg)
        }
        ConfigCommands::Set { key, value } => {
            let mut cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            match key.as_str() {
                "ui.bell" => cfg.ui.bell = parse_bool(&value)?,
                "ui.color" => {
                    cfg.ui.color = value.parse()?;
                }
                "ui.fun" => cfg.ui.fun = parse_bool(&value)?,
                _ => {
                    return Err(ColabError::config(
                        "supported config keys: ui.bell, ui.color, ui.fun",
                    ));
                }
            }
            cfg.save(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            Ok(())
        }
        ConfigCommands::Open => {
            if let Ok(editor) = std::env::var("EDITOR")
                && !editor.trim().is_empty()
            {
                let status = Command::new(editor).arg(&path).status()?;
                if status.success() {
                    return Ok(());
                }
            }
            println!("{}", path.display());
            Ok(())
        }
    }
}

fn handle_bug_report(show_private: bool, _json: bool) -> Result<()> {
    let auth = auth::current_account()?;
    let account = auth.map(|a| {
        serde_json::json!({
            "email": crate::cocli::auth::redaction::redacted_email(&a.email, show_private),
            "name": if show_private { a.name } else { "<redacted>".into() }
        })
    });
    let data = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "auth": account,
        "config_path": if show_private { config::config_path().ok().map(|p| p.display().to_string()) } else { Some("<redacted>".to_string()) },
        "data_dir": if show_private { config::data_dir().ok().map(|p| p.display().to_string()) } else { Some("<redacted>".to_string()) },
        "next_action": "include this JSON when filing an issue"
    });
    let text = serde_json::to_string_pretty(&data)?;
    let redacted = crate::cocli::auth::redaction::redact_sensitive(&text);
    println!("{redacted}");
    Ok(())
}

fn auth_profiles_path() -> Result<PathBuf> {
    Ok(config::data_dir()
        .map_err(|e| ColabError::config(e.to_string()))?
        .join("auth-profiles.toml"))
}

fn load_auth_profiles() -> Result<crate::cocli::auth::profiles::AuthProfiles> {
    crate::cocli::auth::profiles::AuthProfiles::load(&auth_profiles_path()?)
        .map_err(|e| ColabError::config(e.to_string()))
}

fn save_auth_profiles(store: &crate::cocli::auth::profiles::AuthProfiles) -> Result<()> {
    store
        .save(&auth_profiles_path()?)
        .map_err(|e| ColabError::config(e.to_string()))
}

fn redacted_profile(
    profile: &crate::cocli::auth::profiles::AuthProfile,
    show_private: bool,
) -> serde_json::Value {
    serde_json::json!({
        "name": profile.name,
        "account_hint": profile.account_hint.as_deref().map(|s| crate::cocli::auth::profiles::redacted_email(s, show_private)),
        "kind": profile.kind.to_string(),
        "created_at": profile.created_at,
        "last_used_at": profile.last_used_at,
        "storage_backend": format!("{:?}", profile.storage_backend).to_ascii_lowercase(),
        "access_token": "<redacted>",
        "refresh_token": "<redacted>"
    })
}

fn now_rfc3339ish() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn load_slurp(path: &str) -> Result<crate::cocli::slurp::config::SlurpConfig> {
    let body = std::fs::read_to_string(path)?;
    crate::cocli::slurp::config::SlurpConfig::from_toml_str(&body)
        .map_err(|e| ColabError::config(e.to_string()))
}

fn fleet_plan_from_args(
    args: &FleetConfigArgs,
) -> Result<(
    crate::cocli::slurp::config::SlurpConfig,
    crate::cocli::fleet::scheduler::FleetPlan,
    Vec<crate::cocli::fleet::compliance::ComplianceFinding>,
)> {
    let cfg = load_slurp(&args.config)?;
    if args.allow_fallback_account
        && cfg
            .accounts
            .iter()
            .any(|a| !a.kind.allows_fleet() || a.allow_fallback_account)
    {
        return Err(ColabError::config(
            "fallback account rotation is blocked for unknown/free profiles",
        ));
    }
    let findings = crate::cocli::fleet::compliance::validate_slurp(&cfg);
    let plan = crate::cocli::fleet::scheduler::plan(&cfg);
    Ok((cfg, plan, findings))
}

fn refuse_if_needed(findings: &[crate::cocli::fleet::compliance::ComplianceFinding]) -> Result<()> {
    if let Some(finding) = findings
        .iter()
        .find(|f| f.level == crate::cocli::fleet::compliance::ComplianceLevel::Refuse)
    {
        return Err(ColabError::config(format!(
            "{}. Next: {}",
            finding.message, finding.next_action
        )));
    }
    Ok(())
}

fn print_fleet_plan(
    cfg: &crate::cocli::slurp::config::SlurpConfig,
    plan: &crate::cocli::fleet::scheduler::FleetPlan,
    findings: &[crate::cocli::fleet::compliance::ComplianceFinding],
    json: bool,
) -> Result<()> {
    if json {
        return print_value(
            true,
            &serde_json::json!({
                "accounts": cfg.accounts,
                "compliance": findings,
                "plan": plan,
                "cost": {
                    "budget_limit": plan.budget_limit,
                    "exact_provider_cost": null,
                    "note": "budget units only unless provider prices are configured"
                }
            }),
        );
    }
    println!("fleet\t{}", plan.name);
    println!("runtimes\t{}", plan.requested_runtimes);
    println!("shards\t{}", plan.shard_count);
    println!("parallel\t{}", plan.max_parallel_tasks);
    println!("budget\t{}", plan.budget_limit);
    println!("stop\t{}", plan.stop_condition);
    for finding in findings {
        println!(
            "{:?}\t{}\tnext: {}",
            finding.level, finding.message, finding.next_action
        );
    }
    println!("fast path\t{}", plan.fast_path);
    Ok(())
}

fn session_accelerator(args: &SessionNewArgs) -> Result<(Variant, Option<String>)> {
    match (args.gpu.as_ref(), args.tpu.as_ref()) {
        (Some(_), Some(_)) => Err(ColabError::config("choose either --gpu or --tpu, not both")),
        (Some(gpu), None) => Ok((Variant::Gpu, Some(gpu.clone()))),
        (None, Some(tpu)) => Ok((Variant::Tpu, Some(tpu.clone()))),
        (None, None) => Ok((Variant::Cpu, None)),
    }
}

async fn handle_url(config: &ColabConfig, ui: Ui, name: Option<String>, open: bool) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let url = crate::cocli::runtime::session_url(&config.colab_domain, &server.endpoint)
        .map_err(|e| ColabError::config(e.to_string()))?;
    println!("{url}");
    if open {
        open_url(&url)?;
        ui.info("opened browser");
    }
    Ok(())
}

async fn handle_file_edit(
    _config: &ColabConfig,
    _ui: Ui,
    _session: Option<String>,
    _path: String,
) -> Result<()> {
    Err(ColabError::config(
        "fs edit is not wired yet; use `fs pull`, edit locally, then `fs push`",
    ))
}

fn handle_fs_sync(args: FsSyncArgs, ui: Ui, json: bool) -> Result<()> {
    if args.watch {
        return Err(ColabError::config(
            "fs sync --watch needs a file watcher backend; run without --watch for a manifest plan",
        ));
    }
    if !args.dry_run {
        return Err(ColabError::config(
            "fs sync currently supports --dry-run planning; use `fs push` for writes",
        ));
    }
    let plan = local_sync_plan(&args.local, &args.include, &args.exclude, args.delete)?;
    if args.explain && !json {
        println!("fs sync dry-run");
        println!("local: {}", args.local);
        println!("remote: {}", args.remote);
        println!("upload: {} file(s)", plan.upload.len());
        println!("delete remote: {} file(s)", plan.delete_remote.len());
        println!("unchanged: {} file(s)", plan.unchanged);
    } else if !json {
        ui.success("sync dry-run planned");
    }
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn handle_fs_diff(args: FsDiffArgs, _ui: Ui) -> Result<()> {
    let plan = local_sync_plan(&args.local, &args.include, &args.exclude, false)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn local_sync_plan(
    local: &str,
    include: &[String],
    exclude: &[String],
    delete: bool,
) -> Result<crate::cocli::fs::SyncPlan> {
    let mut options = crate::cocli::fs::ManifestOptions {
        include: include.to_vec(),
        ..crate::cocli::fs::ManifestOptions::default()
    };
    if !exclude.is_empty() {
        options.exclude.extend(exclude.iter().cloned());
    }
    let manifest = crate::cocli::fs::FileManifest::build(Path::new(local), &options)
        .map_err(|e| ColabError::config(e.to_string()))?;
    let remote = crate::cocli::fs::FileManifest::default();
    Ok(crate::cocli::fs::diff(&manifest, &remote, delete))
}

async fn compat_transfer(
    args: CompatTransferArgs,
    upload: bool,
    config: &ColabConfig,
    ui: Ui,
) -> Result<()> {
    if upload {
        handle_upload(config, ui, args.session, &args.src, Some(&args.dest)).await
    } else {
        handle_download(config, ui, args.session, &args.src, Some(&args.dest)).await
    }
}

fn migration(ui: &Ui, new: &str) {
    if !ui.quiet {
        println!("moved: use `{new}`");
    }
}

fn runtime_migration_target(cmd: &RuntimeCommands) -> &'static str {
    match cmd {
        RuntimeCommands::Info { backend: true } | RuntimeCommands::BackendInfo => {
            "colab-cli status runtime --backend"
        }
        RuntimeCommands::Info { backend: false } => "colab-cli status runtime",
        RuntimeCommands::Gpu => "colab-cli status runtime --gpu",
        RuntimeCommands::Tpu => "colab-cli status runtime --tpu",
        RuntimeCommands::Versions => "colab-cli status runtime --versions",
        RuntimeCommands::Fit { .. } => "colab-cli status runtime --fit MODEL",
    }
}

fn mount_migration_target(cmd: &MountCommands) -> &'static str {
    match cmd {
        MountCommands::Drive { .. } => "colab-cli fs drive mount",
        MountCommands::List { .. } => "colab-cli fs drive status",
    }
}

fn config_migration_target(cmd: &ConfigCommands) -> &'static str {
    match cmd {
        ConfigCommands::Get => "colab-cli settings get",
        ConfigCommands::Set { .. } => "colab-cli settings set KEY VALUE",
        ConfigCommands::Path => "colab-cli settings path",
        ConfigCommands::Open => "colab-cli settings edit",
    }
}

fn print_value<T: serde::Serialize>(json: bool, value: &T) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

fn continuations_dir(config: &ColabConfig) -> PathBuf {
    config.data_dir.join("continue")
}

fn continuation_path(config: &ColabConfig, name: &str) -> PathBuf {
    continuations_dir(config).join(format!("{name}.json"))
}

fn write_continuation(
    config: &ColabConfig,
    name: &str,
    manifest: &crate::cocli::r#continue::manifest::ContinuationManifest,
) -> Result<()> {
    let dir = continuations_dir(config);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        continuation_path(config, name),
        manifest
            .to_json_pretty()
            .map_err(|e| ColabError::parse(e.to_string()))?,
    )?;
    Ok(())
}

fn read_continuation(
    config: &ColabConfig,
    name: &str,
) -> Result<crate::cocli::r#continue::manifest::ContinuationManifest> {
    let bytes = std::fs::read(continuation_path(config, name))?;
    crate::cocli::r#continue::manifest::ContinuationManifest::from_json(&bytes)
        .map_err(|e| ColabError::parse(e.to_string()))
}

fn newest_continuation(config: &ColabConfig) -> Result<Option<String>> {
    let dir = continuations_dir(config);
    if !dir.exists() {
        return Ok(None);
    }
    let mut newest: Option<(std::time::SystemTime, String)> = None;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let modified = entry.metadata()?.modified()?;
        if newest.as_ref().is_none_or(|(t, _)| modified > *t) {
            newest = Some((modified, name));
        }
    }
    Ok(newest.map(|(_, name)| name))
}

fn git_snapshot() -> crate::cocli::r#continue::manifest::GitSnapshot {
    let commit_hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    let dirty_tree = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| !o.stdout.is_empty());
    crate::cocli::r#continue::manifest::GitSnapshot {
        commit_hash,
        dirty_tree,
    }
}

fn append_audit(line: &str) -> Result<()> {
    let dir = config::data_dir().map_err(|e| ColabError::config(e.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("agent-audit.log"))?;
    writeln!(file, "{} {line}", chrono::Utc::now().to_rfc3339())?;
    Ok(())
}

fn parse_bool(s: &str) -> Result<bool> {
    match s {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ColabError::config("boolean value must be true or false")),
    }
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut cmd = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start"]);
        c
    };
    cmd.arg(url);
    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(ColabError::config("browser open command failed"))
    }
}

async fn handle_login(config: &ColabConfig, ui: Ui) -> Result<()> {
    let pb = ui.spinner("Opening browser for Google sign-in\u{2026}");
    match auth::login(config).await {
        Ok(account) => {
            Ui::spinner_done(pb, &format!("Signed in as {}", account.email));
            ui.print_auth_status(&account.email, &account.name);
            Ok(())
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn handle_server(cmd: ServerCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        ServerCommands::Assign {
            variant,
            accelerator,
            name,
            high_ram,
            keepalive,
        } => {
            handle_assign(
                config,
                ui,
                variant,
                accelerator,
                name,
                shape_from(high_ram),
                keepalive,
            )
            .await
        }
        ServerCommands::Reconfigure {
            name,
            variant,
            accelerator,
            high_ram,
            keepalive,
        } => {
            handle_reconfigure(
                config,
                ui,
                name,
                variant,
                accelerator,
                shape_from(high_ram),
                keepalive,
            )
            .await
        }
        ServerCommands::Ls { available } => {
            if available {
                handle_ls_available(config, ui).await
            } else {
                handle_ls(config, ui).await
            }
        }
        ServerCommands::Rm { name } => handle_rm(config, ui, name).await,
        ServerCommands::Shell { name } => handle_shell(config, ui, name).await,
        ServerCommands::Info { name } => handle_info(config, ui, name).await,
        ServerCommands::Ps { name, interval } => handle_ps(config, ui, name, interval).await,
        ServerCommands::Run { name, command } => handle_run(config, ui, name, command).await,
    }
}

// `colab server run -- <argv>` — stream remote stdout/stderr, propagate exit code
async fn handle_run(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let client = manager.client();

    let exit_code = runner::run_passthrough(client, &server, &command).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

#[inline]
fn shape_from(high_ram: bool) -> Shape {
    if high_ram {
        Shape::HighMem
    } else {
        Shape::Standard
    }
}

async fn handle_assign(
    config: &ColabConfig,
    ui: Ui,
    cli_variant: Option<Variant>,
    cli_accelerator: Option<String>,
    cli_name: Option<String>,
    cli_shape: Shape,
    keepalive: bool,
) -> Result<()> {
    let manager = make_manager(config)?;
    let client = manager.client();

    let ccu = client.get_ccu_info().await.ok();
    let servers = manager.list_local()?;

    let fully_specified = cli_variant.is_some() && cli_name.is_some();

    if fully_specified || ui.quiet {
        let variant = cli_variant.unwrap_or(Variant::Cpu);
        let label = cli_name.unwrap_or_else(|| default_label(variant, cli_accelerator.as_deref()));
        let server = do_assign(
            &manager,
            &ui,
            &ccu,
            label,
            variant,
            cli_accelerator,
            cli_shape,
        )
        .await?;
        if keepalive {
            return run_keepalive_loop(config, ui, server).await;
        }
        return Ok(());
    }

    let auto_connect = if let Some(latest) = latest_server(&servers) {
        let actions = [
            format!("Auto connect ({})", latest.label),
            "New Colab server".to_string(),
        ];
        let choice = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Select action")
            .items(&actions)
            .default(0)
            .interact()
            .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?;
        choice == 0
    } else {
        false
    };

    if auto_connect {
        let Some(server) = latest_server(&servers).cloned() else {
            return Err(ColabError::config("no servers assigned"));
        };
        ui.success(&format!("Connected to '{}'", server.label));
        ui.print_server_status(&server);
        if keepalive {
            return run_keepalive_loop(config, ui, server).await;
        }
        return Ok(());
    }

    let mut accel_choices = vec!["CPU".to_string()];
    if let Some(ref info) = ccu {
        for gpu in &info.eligible_gpus {
            accel_choices.push(format!("{gpu} GPU"));
        }
        for tpu in &info.eligible_tpus {
            accel_choices.push(format!("{tpu} TPU"));
        }
    } else {
        accel_choices.push("GPU".to_string());
        accel_choices.push("TPU".to_string());
    }

    let accel_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Accelerator")
        .items(&accel_choices)
        .default(0)
        .interact()
        .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?;

    let (variant, accelerator) = parse_accel_choice(&accel_choices[accel_idx]);

    let shape_choices = ["Standard RAM", "High-RAM"];
    let shape_idx = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Machine shape")
        .items(&shape_choices)
        .default(if matches!(cli_shape, Shape::HighMem) {
            1
        } else {
            0
        })
        .interact()
        .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?;
    let shape = if shape_idx == 1 {
        Shape::HighMem
    } else {
        Shape::Standard
    };

    let default_name = default_label(variant, accelerator.as_deref());
    let label: String = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Server name")
        .default(default_name)
        .interact_text()
        .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?;

    let server = do_assign(&manager, &ui, &ccu, label, variant, accelerator, shape).await?;
    if keepalive {
        return run_keepalive_loop(config, ui, server).await;
    }
    Ok(())
}

async fn do_assign(
    manager: &ServerManager,
    ui: &Ui,
    ccu: &Option<crate::cocli::session::model::CcuInfo>,
    label: String,
    variant: Variant,
    accelerator: Option<String>,
    shape: Shape,
) -> Result<StoredServer> {
    if let Some(info) = ccu {
        println!();
        ui.info(&format!(
            "Available: {:.2} compute units",
            info.current_balance
        ));
        ui.info(&format!(
            "Usage rate: ~{:.2} CCU/hr based on current sessions",
            info.consumption_rate_hourly
        ));
        println!();
    }

    let pb = ui.spinner(&format!(
        "Assigning {} server ({})\u{2026}",
        variant.display_name(),
        shape.display_name()
    ));
    match manager.assign(label, variant, accelerator, shape).await {
        Ok(outcome) => {
            Ui::spinner_done(pb, "Assigned");
            println!();
            ui.success(&format!("runtime warmed up: {}", outcome.server.label));
            if outcome.shape_mismatch {
                ui.warn(&format!(
                    "Requested {} but Colab provisioned {}. Your account tier may not allow {} shape.",
                    outcome.requested_shape,
                    outcome.reported_shape.unwrap_or(Shape::Standard),
                    outcome.requested_shape,
                ));
            }
            ui.print_server_status(&outcome.server);
            Ok(outcome.server)
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn handle_reconfigure(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    variant: Option<Variant>,
    accelerator: Option<String>,
    shape: Shape,
    keepalive: bool,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?.clone();
    let variant = variant.unwrap_or(server.variant);

    let pb = ui.spinner(&format!(
        "Reconfiguring '{}' \u{2192} {} / {}\u{2026}",
        server.label,
        variant.display_name(),
        shape.display_name()
    ));
    match manager
        .reconfigure(server.id, variant, accelerator, shape)
        .await
    {
        Ok(outcome) => {
            Ui::spinner_done(pb, "Reconfigured");
            ui.success(&format!("'{}' reconfigured", outcome.server.label));
            if outcome.shape_mismatch {
                ui.warn(&format!(
                    "Requested {} but Colab provisioned {}. Your account tier may not allow {} shape.",
                    outcome.requested_shape,
                    outcome.reported_shape.unwrap_or(Shape::Standard),
                    outcome.requested_shape,
                ));
            }
            ui.print_server_status(&outcome.server);
            if keepalive {
                return run_keepalive_loop(config, ui, outcome.server).await;
            }
            Ok(())
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn run_keepalive_loop(config: &ColabConfig, ui: Ui, mut server: StoredServer) -> Result<()> {
    let manager = make_manager(config)?;
    let client = manager.client();

    println!();
    ui.success(&format!(
        "Keep-alive active for '{}' — press Ctrl-C to stop",
        server.label
    ));
    ui.info("Pinging every 4 minutes · auto-refreshing tokens");
    println!();

    let cancel = tokio::signal::ctrl_c();
    tokio::pin!(cancel);

    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(4 * 60));
    ping_interval.tick().await;

    loop {
        tokio::select! {
            _ = &mut cancel => {
                println!();
                ui.info("Keep-alive stopped");
                return Ok(());
            }
            _ = ping_interval.tick() => {
                let remaining = server.token_expires_at - chrono::Utc::now();
                if remaining.num_seconds() < 10 * 60 {
                    match manager.refresh(server.id).await {
                        Ok(updated) => {
                            server = updated;
                            let ts = chrono::Local::now().format("%H:%M:%S");
                            ui.info(&format!("[{ts}] token refreshed"));
                        }
                        Err(e) => ui.warn(&format!("token refresh failed: {e}")),
                    }
                }

                match client.send_keep_alive(&server.endpoint).await {
                    Ok(()) => {
                        let ts = chrono::Local::now().format("%H:%M:%S");
                        ui.info(&format!("[{ts}] ping ok"));
                    }
                    Err(e) => ui.warn(&format!("ping failed: {e}")),
                }
            }
        }
    }
}

async fn handle_ps(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    _interval_ms: u64,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let client = manager.client();

    if ui.quiet {
        return handle_ps_oneshot(client, &server, &ui).await;
    }

    // tiny spinner before the TUI takes over
    let pb = ui.spinner("Opening monitor\u{2026}");

    // pick a system monitor: bpytop > btop > bashtop > htop. install bpytop
    // if nothing is around. exec replaces the shell so the process tree's
    // clean — when the user quits, the jupyter terminal closes naturally
    // and our cleanup guard reaps it.
    let bootstrap = r#"
_CPS_pick() {
  command -v bpytop 2>/dev/null \
    || command -v btop 2>/dev/null \
    || command -v bashtop 2>/dev/null \
    || command -v htop 2>/dev/null
}
_CPS=$(_CPS_pick)
if [ -z "$_CPS" ]; then
  clear
  printf '  Preparing monitor\xe2\x80\xa6\r'
  { pip install --quiet --disable-pip-version-check bpytop; } >/dev/null 2>&1
  _CPS=$(_CPS_pick)
fi
if [ -n "$_CPS" ]; then
  clear
  exec "$_CPS"
else
  printf '  Monitor unavailable on this runtime.\n' >&2
  exit 2
fi
"#;
    // base64 the bootstrap so quoting/control chars can't confuse the PTY
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bootstrap.trim());
    let remote_cmd = format!("eval \"$(printf '%s' '{encoded}' | base64 -d)\"");

    // close the spinner before the alt-screen takeover; indicatif fights it
    Ui::spinner_done(pb, "");

    let result = runner::run_remote_tui(client, &server, &remote_cmd).await;

    // reconnect failures come back as OAuth-style errors (transient network)
    match result {
        Ok(()) => Ok(()),
        Err(ColabError::OAuth(msg)) if msg.contains("could not reattach") => {
            ui.warn("Monitor disconnected — the remote session may still be running");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

async fn handle_ps_oneshot(client: &ColabClient, server: &StoredServer, ui: &Ui) -> Result<()> {
    let cmd = r#"
echo '<<<UNAME>>>'; uname -srm
echo '<<<CPU>>>'; (nproc 2>/dev/null && grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')
echo '<<<MEM>>>'; free -h 2>/dev/null | awk 'NR==2{print $2"\t"$3"\t"$4}'
echo '<<<DISK>>>'; df -h / 2>/dev/null | awk 'NR==2{print $2"\t"$3"\t"$4"\t"$5}'
echo '<<<GPU>>>'; (nvidia-smi --query-gpu=name,memory.total,memory.used --format=csv,noheader 2>/dev/null || echo none)
echo '<<<UPTIME>>>'; uptime -p 2>/dev/null || uptime
"#;
    let output = runner::capture_remote_command(client, server, cmd).await?;
    ui.print_system_info(&server.label, &output);
    Ok(())
}

async fn handle_ls(config: &ColabConfig, ui: Ui) -> Result<()> {
    let manager = make_manager(config)?;
    let pb = ui.spinner("Fetching server list\u{2026}");
    match manager.list().await {
        Ok((servers, removed)) => {
            Ui::spinner_done(pb, "Done");
            if removed > 0 {
                ui.warn(&format!(
                    "{removed} server(s) removed externally since last sync"
                ));
            }
            ui.print_server_list(&servers);
            Ok(())
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn handle_ls_available(config: &ColabConfig, ui: Ui) -> Result<()> {
    let client = make_client(config)?;
    let pb = ui.spinner("Fetching accelerator info\u{2026}");
    match client.get_ccu_info().await {
        Ok(info) => {
            Ui::spinner_done(pb, "Done");
            ui.print_accelerators(&info);
            Ok(())
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn handle_rm(config: &ColabConfig, ui: Ui, name: Option<String>) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let label = server.label.clone();
    let id = server.id;

    let pb = ui.spinner(&format!("Removing '{label}'\u{2026}"));
    match manager.remove(id).await {
        Ok(()) => {
            Ui::spinner_done(pb, "Removed");
            ui.success(&format!("Server '{label}' removed"));
            Ok(())
        }
        Err(e) => {
            Ui::spinner_fail(pb, &e.to_string());
            Err(e)
        }
    }
}

async fn handle_shell(config: &ColabConfig, ui: Ui, name: Option<String>) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;

    let server = match resolve_server(&servers, name.as_deref()) {
        Ok(s) => s.clone(),
        Err(_) if name.is_none() => {
            ui.info("No server assigned. Assigning a default CPU server\u{2026}");
            let pb = ui.spinner("Assigning CPU server\u{2026}");
            match manager
                .assign("Colab CPU".to_string(), Variant::Cpu, None, Shape::Standard)
                .await
            {
                Ok(outcome) => {
                    Ui::spinner_done(pb, &format!("Assigned '{}'", outcome.server.label));
                    outcome.server
                }
                Err(e) => {
                    Ui::spinner_fail(pb, &e.to_string());
                    return Err(e);
                }
            }
        }
        Err(e) => return Err(e),
    };

    let server = ensure_fresh_token(&manager, &server, &ui).await?;

    ui.info(&format!(
        "Connecting to '{}' ({})\u{2026}",
        server.label,
        server.variant.display_name()
    ));

    let client = manager.client();

    // refresher the shell's keepalive uses to rotate the token on long sessions
    let refresher: runner::TokenRefresher = {
        let config = config.clone();
        let server_id = server.id;
        std::sync::Arc::new(move || {
            let config = config.clone();
            Box::pin(async move {
                let manager = make_manager(&config)?;
                manager.refresh(server_id).await
            })
        })
    };

    runner::run_shell(client, &server, None, Some(refresher)).await
}

async fn handle_info(config: &ColabConfig, ui: Ui, name: Option<String>) -> Result<()> {
    match auth::current_account()? {
        Some(account) => ui.print_auth_status(&account.email, &account.name),
        None => {
            ui.print_auth_not_signed_in();
            return Ok(());
        }
    }

    let manager = make_manager(config)?;
    let servers = manager.list_local()?;

    match resolve_server(&servers, name.as_deref()) {
        Ok(s) => {
            println!();
            ui.print_server_status(s);
        }
        Err(_) => ui.info("No servers assigned."),
    }

    let client = manager.client();
    if let Ok(ccu) = client.get_ccu_info().await {
        println!();
        ui.print_usage(&ccu);
    }

    Ok(())
}

async fn handle_file(cmd: FileCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        FileCommands::Upload { name, src, dest } => {
            handle_upload(config, ui, name, &src, dest.as_deref()).await
        }
        FileCommands::Download { name, src, dest } => {
            handle_download(config, ui, name, &src, dest.as_deref()).await
        }
        FileCommands::Ls { name, args } => handle_file_ls(config, ui, name, args).await,
        FileCommands::Cp { name, args } => handle_file_cp(config, ui, name, args).await,
        FileCommands::Rm { name, args } => handle_file_rm(config, ui, name, args).await,
    }
}

async fn handle_upload(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    src: &str,
    dest: Option<&str>,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;

    // expand `~` so `upload ~/data.csv` just works
    let expanded_src = expand_tilde(src);
    let path = expanded_src.as_path();
    if !path.exists() {
        return Err(ColabError::config(format!("file not found: {src}")));
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload");

    // dest: none → /content/<name>; ends with / → treat as dir; else literal
    let remote_path: String = match dest {
        None => format!("/content/{file_name}"),
        Some(d) if d.ends_with('/') => format!("{d}{file_name}"),
        Some(d) => d.to_string(),
    };

    let file_size = std::fs::metadata(path)?.len();

    let pb = if !ui.quiet && file_size > 1024 * 1024 {
        let pb = indicatif::ProgressBar::new(file_size);
        if let Ok(style) = indicatif::ProgressStyle::with_template(
            "{spinner:.cyan} Uploading [{bar:30}] {bytes}/{total_bytes} ({eta})",
        ) {
            pb.set_style(style.progress_chars("\u{2588}\u{2593}\u{2591}"));
        }
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    } else {
        ui.spinner(&format!("Uploading {file_name}\u{2026}"))
    };

    let pb_clone = pb.clone();
    let progress = move |bytes_read: u64| {
        if let Some(ref pb) = pb_clone {
            pb.set_position(bytes_read);
        }
    };

    let client = manager.client();
    match client
        .upload_file_streaming(
            &server.proxy_url,
            &server.proxy_token,
            &remote_path,
            path,
            progress,
        )
        .await
    {
        Ok(()) => {
            if let Some(pb) = pb {
                pb.finish_and_clear();
            }
            ui.success(&format!("{src} \u{2192} {remote_path}"));
            Ok(())
        }
        Err(e) => {
            if let Some(pb) = pb {
                pb.finish_with_message(format!("\u{2717} {e}"));
            }
            Err(e)
        }
    }
}

async fn handle_download(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    src: &str,
    dest: Option<&str>,
) -> Result<()> {
    use std::path::{Path, PathBuf};

    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let mut server = ensure_fresh_token(&manager, server, &ui).await?;
    let client = manager.client().clone();

    // Background keep-alive for the duration of the download: sends the
    // Colab tunnel ping every 4min so Google doesn't reclaim the runtime
    // mid-transfer. Token rotation is handled inline between files below.
    let keepalive_client = client.clone();
    let keepalive_endpoint = server.endpoint.clone();
    let keepalive_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(4 * 60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let _ = keepalive_client.send_keep_alive(&keepalive_endpoint).await;
        }
    });
    let _keepalive_guard = KeepaliveGuard(keepalive_handle);

    let remote = src.to_string();
    let entry = client
        .stat_contents(&server.proxy_url, &server.proxy_token, &remote)
        .await?;

    // Resolve the local destination:
    //   - none → ./<basename>
    //   - existing dir → <dest>/<basename>
    //   - ends with `/` → create dir, then <dest>/<basename>
    //   - else → literal path
    let remote_basename = Path::new(remote.trim_end_matches('/'))
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&entry.name)
        .to_string();

    let dest_expanded = dest.map(expand_tilde);
    let local_root: PathBuf = match dest_expanded.as_deref() {
        None => PathBuf::from(&remote_basename),
        Some(p) if p.is_dir() => p.join(&remote_basename),
        Some(p) if dest.is_some_and(|d| d.ends_with('/')) => {
            std::fs::create_dir_all(p)?;
            p.join(&remote_basename)
        }
        Some(p) => p.to_path_buf(),
    };

    if entry.is_directory() {
        let pb = ui.spinner(&format!("Downloading {remote}\u{2026}"));
        let mut stats = DownloadStats::default();
        let result = download_directory_recursive(
            &manager,
            &client,
            &mut server,
            &remote,
            &local_root,
            &mut stats,
            &ui,
        )
        .await;
        match result {
            Ok(()) => {
                Ui::spinner_done(pb, "Done");
                ui.success(&format!(
                    "{remote} \u{2192} {} ({} files, {} bytes)",
                    local_root.display(),
                    stats.files,
                    stats.bytes
                ));
                Ok(())
            }
            Err(e) => {
                Ui::spinner_fail(pb, &e.to_string());
                Err(e)
            }
        }
    } else if entry.is_file() {
        let total_hint = entry.size.unwrap_or(0);
        let pb = if !ui.quiet && total_hint > 1024 * 1024 {
            let pb = indicatif::ProgressBar::new(total_hint);
            if let Ok(style) = indicatif::ProgressStyle::with_template(
                "{spinner:.cyan} Downloading [{bar:30}] {bytes}/{total_bytes} ({eta})",
            ) {
                pb.set_style(style.progress_chars("\u{2588}\u{2593}\u{2591}"));
            }
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        } else {
            ui.spinner(&format!("Downloading {remote_basename}\u{2026}"))
        };

        let pb_clone = pb.clone();
        let progress = move |bytes: u64| {
            if let Some(ref pb) = pb_clone {
                pb.set_position(bytes);
            }
        };

        match client
            .download_file_streaming(
                &server.proxy_url,
                &server.proxy_token,
                &remote,
                &local_root,
                progress,
            )
            .await
        {
            Ok(bytes) => {
                if let Some(pb) = pb {
                    pb.finish_and_clear();
                }
                ui.success(&format!(
                    "{remote} \u{2192} {} ({} bytes)",
                    local_root.display(),
                    bytes
                ));
                Ok(())
            }
            Err(e) => {
                if let Some(pb) = pb {
                    pb.finish_with_message(format!("\u{2717} {e}"));
                }
                Err(e)
            }
        }
    } else {
        Err(ColabError::config(format!(
            "unsupported remote entry type: {}",
            entry.kind
        )))
    }
}

#[derive(Default)]
struct DownloadStats {
    files: u64,
    bytes: u64,
}

struct KeepaliveGuard(tokio::task::JoinHandle<()>);

impl Drop for KeepaliveGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Recursively download a remote directory into `local_root`. Refreshes
/// the proxy token inline if it drifts under 5 minutes of life; stats are
/// accumulated into the caller's `DownloadStats` for the final summary.
fn download_directory_recursive<'a>(
    manager: &'a ServerManager,
    client: &'a ColabClient,
    server: &'a mut StoredServer,
    remote_dir: &'a str,
    local_dir: &'a std::path::Path,
    stats: &'a mut DownloadStats,
    ui: &'a Ui,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        std::fs::create_dir_all(local_dir)?;

        let entries = client
            .list_directory(&server.proxy_url, &server.proxy_token, remote_dir)
            .await?;

        for child in entries {
            // Rotate the token between children if it's about to expire.
            // A single directory walk can take many minutes; the proxy
            // token has a short TTL, so refresh proactively rather than
            // eating a 401 mid-walk.
            let remaining = server.token_expires_at - chrono::Utc::now();
            if remaining.num_seconds() < 5 * 60
                && let Ok(updated) = manager.refresh(server.id).await
            {
                *server = updated;
            }

            let child_local = local_dir.join(&child.name);
            if child.is_directory() {
                download_directory_recursive(
                    manager,
                    client,
                    server,
                    &child.path,
                    &child_local,
                    stats,
                    ui,
                )
                .await?;
            } else if child.is_file() {
                let bytes = client
                    .download_file_streaming(
                        &server.proxy_url,
                        &server.proxy_token,
                        &child.path,
                        &child_local,
                        |_| {},
                    )
                    .await?;
                stats.files += 1;
                stats.bytes += bytes;
                ui.info(&format!("  {} ({} bytes)", child.path, bytes));
            }
        }

        Ok(())
    })
}

// expand `~` and `~/foo`. doesn't handle `~user`.
fn expand_tilde(p: &str) -> std::path::PathBuf {
    if p == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    std::path::PathBuf::from(p)
}

async fn handle_file_ls(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    // default to a long listing of /content if the user gave nothing
    let args = if args.is_empty() {
        vec!["-lah".to_string(), "/content".to_string()]
    } else {
        args
    };
    run_remote_tool(config, ui, name, "ls", args).await
}

async fn handle_file_cp(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    run_remote_tool(config, ui, name, "cp", args).await
}

async fn handle_file_rm(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    args: Vec<String>,
) -> Result<()> {
    run_remote_tool(config, ui, name, "rm", args).await
}

// shared: resolve server → ship `<tool> <args...>` via run_passthrough → exit
async fn run_remote_tool(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    tool: &str,
    args: Vec<String>,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let client = manager.client();

    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(tool.to_string());
    argv.extend(args);

    let exit_code = runner::run_passthrough(client, &server, &argv).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn resolve_server<'a>(servers: &'a [StoredServer], name: Option<&str>) -> Result<&'a StoredServer> {
    match name {
        Some("-") => servers
            .iter()
            .max_by_key(|s| s.date_assigned)
            .ok_or_else(|| ColabError::config("no active session - run `colab-cli session list`")),
        Some(n) => servers
            .iter()
            .find(|s| s.label == n || s.endpoint == n || s.id.to_string() == n)
            .ok_or_else(|| ColabError::ServerNotFound {
                endpoint: n.to_string(),
            }),
        None => servers
            .iter()
            .max_by_key(|s| s.date_assigned)
            .ok_or_else(|| ColabError::config("no servers assigned")),
    }
}

fn latest_server(servers: &[StoredServer]) -> Option<&StoredServer> {
    servers.iter().max_by_key(|s| s.date_assigned)
}

async fn ensure_fresh_token(
    manager: &ServerManager,
    server: &StoredServer,
    ui: &Ui,
) -> Result<StoredServer> {
    let remaining = server.token_expires_at - chrono::Utc::now();
    if remaining.num_seconds() < 5 * 60 {
        let pb = ui.spinner("Refreshing connection token\u{2026}");
        match manager.refresh(server.id).await {
            Ok(updated) => {
                Ui::spinner_done(pb, "Token refreshed");
                Ok(updated)
            }
            Err(e) => {
                Ui::spinner_fail(pb, &e.to_string());
                Err(e)
            }
        }
    } else {
        Ok(server.clone())
    }
}

fn default_label(variant: Variant, accelerator: Option<&str>) -> String {
    match accelerator {
        Some(acc) if !acc.is_empty() => format!("Colab {} {acc}", variant.display_name()),
        _ => format!("Colab {}", variant.display_name()),
    }
}

fn parse_accel_choice(choice: &str) -> (Variant, Option<String>) {
    if choice == "CPU" {
        return (Variant::Cpu, None);
    }
    if choice == "GPU" {
        return (Variant::Gpu, None);
    }
    if choice == "TPU" {
        return (Variant::Tpu, None);
    }
    if let Some(acc) = choice.strip_suffix(" GPU") {
        return (Variant::Gpu, Some(acc.to_string()));
    }
    if let Some(acc) = choice.strip_suffix(" TPU") {
        return (Variant::Tpu, Some(acc.to_string()));
    }
    (Variant::Cpu, None)
}

fn make_manager(config: &ColabConfig) -> Result<ServerManager> {
    let client = make_client(config)?;
    Ok(ServerManager::new(client, config))
}

fn make_client(config: &ColabConfig) -> Result<ColabClient> {
    ColabClient::new(config, {
        let config = config.clone();
        move || {
            let config = config.clone();
            async move { auth::get_access_token(&config).await }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accel_choice_cpu() {
        assert_eq!(parse_accel_choice("CPU"), (Variant::Cpu, None));
    }

    #[test]
    fn parse_accel_choice_gpu_bare() {
        assert_eq!(parse_accel_choice("GPU"), (Variant::Gpu, None));
    }

    #[test]
    fn parse_accel_choice_tpu_bare() {
        assert_eq!(parse_accel_choice("TPU"), (Variant::Tpu, None));
    }

    #[test]
    fn parse_accel_choice_named_gpu() {
        assert_eq!(
            parse_accel_choice("T4 GPU"),
            (Variant::Gpu, Some("T4".to_string()))
        );
        assert_eq!(
            parse_accel_choice("A100 GPU"),
            (Variant::Gpu, Some("A100".to_string()))
        );
    }

    #[test]
    fn parse_accel_choice_named_tpu() {
        assert_eq!(
            parse_accel_choice("v2-8 TPU"),
            (Variant::Tpu, Some("v2-8".to_string()))
        );
    }

    #[test]
    fn default_label_shapes() {
        assert_eq!(default_label(Variant::Cpu, None), "Colab CPU");
        assert_eq!(default_label(Variant::Gpu, Some("T4")), "Colab GPU T4");
        assert_eq!(default_label(Variant::Tpu, Some("v2-8")), "Colab TPU v2-8");
    }

    #[test]
    fn shape_from_flag() {
        assert_eq!(shape_from(false), Shape::Standard);
        assert_eq!(shape_from(true), Shape::HighMem);
    }

    #[test]
    fn drive_mount_cell_uses_colab_kernel_code() {
        let code = drive_mount_cell("/content/drive");
        assert!(code.contains("from google.colab import drive"));
        assert!(code.contains("drive.mount(\"/content/drive\", force_remount=False)"));
        assert!(!code.contains("python -c"));
    }

    #[test]
    fn drive_status_parse_handles_mounted_not_mounted_and_unknown() {
        let mounted = parse_drive_status("mounted\npath=/content/drive", "/content/drive");
        assert_eq!(mounted.mounted, Some(true));
        assert!(mounted.next_action.is_none());

        let not_mounted = parse_drive_status("not_mounted\npath=/content/drive", "/content/drive");
        assert_eq!(not_mounted.mounted, Some(false));
        assert_eq!(
            not_mounted.next_action.as_deref(),
            Some("colab-cli fs drive mount")
        );

        let unknown = parse_drive_status("", "/content/drive");
        assert_eq!(unknown.mounted, None);
        assert_eq!(
            unknown.next_action.as_deref(),
            Some("colab-cli status check")
        );
    }

    #[test]
    fn drive_kernel_traceback_gets_friendly_error() {
        let raw = "AttributeError: 'NoneType' object has no attribute 'kernel'";
        let Some(ColabError::Drive {
            kind,
            message,
            next_action,
            raw,
        }) = classify_drive_error(raw)
        else {
            panic!("expected drive error");
        };
        assert_eq!(kind, "drive_kernel_context_required");
        assert_eq!(
            message,
            "Drive mount needs a Colab kernel session, not a plain Python process"
        );
        assert_eq!(next_action.as_deref(), Some("colab-cli session url --open"));
        assert!(raw.as_deref().unwrap_or_default().contains("kernel"));
    }

    #[test]
    fn drive_auth_request_gets_browser_approval_error() {
        let raw = "google.colab._message.blocking_request request_auth";
        let Some(ColabError::Drive {
            kind,
            message,
            next_action,
            ..
        }) = classify_drive_error(raw)
        else {
            panic!("expected drive error");
        };
        assert_eq!(kind, "drive_browser_approval_required");
        assert_eq!(message, "Drive needs browser approval");
        assert_eq!(next_action.as_deref(), Some("colab-cli session url --open"));
    }
}
