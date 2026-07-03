use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{CommandFactory, Parser};

use colab_cli::auth;
use colab_cli::cli::{
    AgentCommands, AuthCommands, Cli, Commands, CompatTransferArgs, ConfigCommands,
    ContinueCommands, DoctorCommands, EnvCommands, ExecCommands, FileCommands, FsCommands,
    FsDiffArgs, FsSyncArgs, MountCommands, RuntimeCommands, ServerCommands, SessionCommands,
    SessionNameArg, SessionNewArgs, ToolsCommands,
};
use colab_cli::client::ColabClient;
use colab_cli::client::api::{Shape, Variant};
use colab_cli::config::ColabConfig;
use colab_cli::error::{ColabError, Result};
use colab_cli::server::ServerManager;
use colab_cli::server::storage::StoredServer;
use colab_cli::shell;
use colab_cli::ui::Ui;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let color_choice: cocli_core::ColorChoice = cli.color.parse().unwrap_or_default();
    let use_color = color_choice.enabled(
        std::env::var_os("NO_COLOR").is_some(),
        std::env::var_os("CI").is_some(),
        cli.quiet,
        cli.json,
    );
    colored::control::set_override(use_color);
    let ring_bell =
        cocli_core::terminal_bell_allowed(cli.bell, std::env::var_os("CI").is_some(), cli.quiet);
    let ui = Ui::new(cli.quiet);

    if let Err(e) = run(cli, ui).await {
        ui.error(&e.to_string());
        if ring_bell {
            eprint!("\x07");
        }

        match &e {
            ColabError::NotAuthenticated => {
                eprintln!("  Run `colab-cli auth login` to sign in.");
            }
            ColabError::TooManyAssignments => {
                eprintln!("  Run `colab-cli server rm` to remove one.");
            }
            _ => {}
        }

        std::process::exit(1);
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
        Commands::Auth { command } => match command {
            AuthCommands::Login => {
                let config = ColabConfig::load(cli.quiet)?;
                handle_login(&config, ui).await
            }
            AuthCommands::Logout => {
                auth::logout()?;
                ui.success("Signed out. Credentials cleared.");
                Ok(())
            }
        },
        Commands::Session { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(command, &config, ui).await
        }
        Commands::Exec { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_exec(command, &config, ui).await
        }
        Commands::Fs { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_fs(command, &config, ui).await
        }
        Commands::Mount { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_mount(command, &config, ui).await
        }
        Commands::Env { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_env(command, &config, ui).await
        }
        Commands::Runtime { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_runtime(command, &config, ui, json).await
        }
        Commands::Tools { command } => handle_tools(command, ui, json),
        Commands::Agent { command } => handle_agent(command, ui, json),
        Commands::Continue { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_continue(command, &config, ui, json).await
        }
        Commands::Config { command } => handle_config(command, json),
        Commands::Doctor { command } => handle_doctor(command, ui, json),
        Commands::Server { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_server(command, &config, ui).await
        }
        Commands::File { command } => {
            let config = ColabConfig::load(cli.quiet)?;
            handle_file(command, &config, ui).await
        }
        Commands::CompatNew(args) => {
            migration(&ui, "colab new", "colab-cli session new");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::New(args), &config, ui).await
        }
        Commands::CompatSessions => {
            migration(&ui, "colab sessions", "colab-cli session list");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::List, &config, ui).await
        }
        Commands::CompatStatus(arg) => {
            migration(&ui, "colab status", "colab-cli session status");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::Status(arg), &config, ui).await
        }
        Commands::CompatStop(arg) => {
            migration(&ui, "colab stop", "colab-cli session stop");
            let config = ColabConfig::load(cli.quiet)?;
            handle_session(SessionCommands::Stop(arg), &config, ui).await
        }
        Commands::CompatUpload(args) => {
            migration(
                &ui,
                "colab upload LOCAL REMOTE",
                "colab-cli fs push LOCAL REMOTE",
            );
            let config = ColabConfig::load(cli.quiet)?;
            compat_transfer(args, true, &config, ui).await
        }
        Commands::CompatDownload(args) => {
            migration(
                &ui,
                "colab download REMOTE LOCAL",
                "colab-cli fs pull REMOTE LOCAL",
            );
            let config = ColabConfig::load(cli.quiet)?;
            compat_transfer(args, false, &config, ui).await
        }
        Commands::Completions { .. } => unreachable!(),
    }
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
            handle_info(config, ui, session).await
        }
        SessionCommands::Stop(SessionNameArg { session }) => handle_rm(config, ui, session).await,
        SessionCommands::Url { session, open } => handle_url(config, ui, session, open).await,
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
    }
}

async fn handle_fs(cmd: FsCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
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
        FsCommands::Sync(args) => handle_fs_sync(args, ui),
        FsCommands::Diff(args) => handle_fs_diff(args, ui),
    }
}

async fn handle_mount(cmd: MountCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        MountCommands::Drive { session, path } => {
            let code = cocli_colab::drive_mount_python(&path);
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-c".into(), code],
            )
            .await
        }
        MountCommands::List { session } => {
            handle_run(config, ui, session, vec!["mount".into()]).await
        }
        MountCommands::Check { session } => {
            handle_run(
                config,
                ui,
                session,
                vec![
                    "python".into(),
                    "-c".into(),
                    "import os; print(os.path.ismount('/content/drive'))".into(),
                ],
            )
            .await
        }
    }
}

async fn handle_env(cmd: EnvCommands, config: &ColabConfig, ui: Ui) -> Result<()> {
    match cmd {
        EnvCommands::Install { packages, session } => {
            if packages.is_empty() {
                return Err(ColabError::config("env install needs at least one package"));
            }
            handle_run(
                config,
                ui,
                session,
                cocli_colab::pip_install_command(&packages),
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
        EnvCommands::Doctor { session } => {
            handle_run(
                config,
                ui,
                session,
                vec![
                    "bash".into(),
                    "-lc".into(),
                    "python -V && python -m pip --version && (nvidia-smi || true)".into(),
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
        RuntimeCommands::Info => handle_info(config, ui, None).await,
        RuntimeCommands::BackendInfo | RuntimeCommands::Versions => {
            let data = serde_json::json!({
                "apt": cocli_colab::backend_info_url("apt-list.txt"),
                "pip": cocli_colab::backend_info_url("pip-freeze.txt"),
                "note": "backend-info can lag production runtimes by one or two days"
            });
            print_value(json, &data)
        }
        RuntimeCommands::Gpu => {
            ui.info("GPU details require a session; use `colab-cli exec py --code \"import torch; print(torch.cuda.get_device_name(0))\"`.");
            Ok(())
        }
        RuntimeCommands::Tpu => {
            ui.info(
                "TPU details require a session; use runtime backend-info for package baselines.",
            );
            Ok(())
        }
    }
}

fn handle_tools(cmd: ToolsCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        ToolsCommands::List { json: local_json } => {
            let specs = cocli_tools::ToolRegistry::specs();
            if json || local_json {
                print_value(true, &specs)
            } else {
                for spec in specs {
                    println!(
                        "{}\t{:?}\tsession={}\tnetwork={}\tdry_run={}",
                        spec.name,
                        spec.risk,
                        spec.requires_session,
                        spec.requires_network,
                        spec.dry_run
                    );
                }
                Ok(())
            }
        }
        ToolsCommands::Inspect {
            tool_name,
            json: local_json,
        } => {
            let spec = cocli_tools::ToolRegistry::inspect(&tool_name)
                .map_err(|e| ColabError::config(e.to_string()))?;
            print_value(json || local_json, &spec)
        }
        ToolsCommands::Run {
            tool_name,
            input_json,
            yes,
        } => {
            let input: serde_json::Value = serde_json::from_str(&input_json)?;
            let output = cocli_tools::ToolRegistry::run_plan(&tool_name, input, yes)
                .map_err(|e| ColabError::config(e.to_string()))?;
            print_value(true, &output)
        }
        ToolsCommands::Install { extension } => {
            ui.info(&format!(
                "extension install is not implemented yet; built-in tools are available (`{extension}` was not changed)"
            ));
            Ok(())
        }
    }
}

fn handle_agent(cmd: AgentCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        AgentCommands::Tools => handle_tools(ToolsCommands::List { json }, ui, json),
        AgentCommands::Plan { goal, out } => {
            let plan = format!(
                "goal = {goal:?}\nconfirm_required = true\n\n[[steps]]\ntool = \"doctor\"\ninput = {{}}\n"
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
        AgentCommands::Mcp { stdio } => {
            if !stdio {
                return Err(ColabError::config(
                    "agent mcp currently supports --stdio only",
                ));
            }
            #[cfg(feature = "mcp")]
            {
                println!(r#"{{"jsonrpc":"2.0","method":"tools/list_changed","params":{{}}}}"#);
                Ok(())
            }
            #[cfg(not(feature = "mcp"))]
            {
                Err(ColabError::config(
                    "MCP adapter is behind `--features mcp`; rebuild with that feature",
                ))
            }
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
            let mut manifest =
                cocli_protocol::ContinuationManifest::new(chrono::Utc::now().to_rfc3339(), &name);
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
            let manifest = cocli_protocol::ContinuationManifest::from_json(&bytes)
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
            let days = cocli_core::parse_days(&older_than)
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
        } => {
            let manifest = read_continuation(config, &name)?;
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

            let mut steps = Vec::new();
            if replay_all {
                steps.extend(manifest.executed_steps.clone());
            }
            steps.extend(manifest.pending_steps.clone());

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
    }
}

fn handle_config(cmd: ConfigCommands, json: bool) -> Result<()> {
    let path = cocli_core::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    match cmd {
        ConfigCommands::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigCommands::Get => {
            let cfg = cocli_core::CocliConfig::load(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            print_value(json, &cfg)
        }
        ConfigCommands::Set { key, value } => {
            let mut cfg = cocli_core::CocliConfig::load(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            match key.as_str() {
                "ui.bell" => cfg.ui.bell = parse_bool(&value)?,
                "ui.color" => {
                    cfg.ui.color = value
                        .parse()
                        .map_err(|e: cocli_core::CoreError| ColabError::config(e.to_string()))?;
                }
                _ => {
                    return Err(ColabError::config(
                        "supported config keys: ui.bell, ui.color",
                    ));
                }
            }
            cfg.save(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            Ok(())
        }
    }
}

fn handle_doctor(cmd: Option<DoctorCommands>, ui: Ui, json: bool) -> Result<()> {
    let auth_state = auth::current_account()?.map(|a| a.email);
    let data = match cmd {
        None => serde_json::json!({
            "auth": auth_state,
            "config_path": cocli_core::config_path().ok().map(|p| p.display().to_string()),
            "unsafe_code": "forbidden by workspace lints"
        }),
        Some(DoctorCommands::Auth) => serde_json::json!({ "auth": auth_state }),
        Some(DoctorCommands::Mounts) => serde_json::json!({
            "note": "mount checks require a live session; use `colab-cli mount check --session NAME`"
        }),
        Some(DoctorCommands::Perf) => serde_json::json!({
            "budgets": {
                "help_ms": 80,
                "config_load_ms": 5,
                "manifest_diff_files": 10000
            },
            "bench": "cargo bench --workspace"
        }),
    };
    if json {
        print_value(true, &data)
    } else {
        println!("cocli ▸ fast path found");
        ui.success("doctor checks complete");
        println!("{}", serde_json::to_string_pretty(&data)?);
        Ok(())
    }
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
    let url = cocli_colab::session_url(&config.colab_domain, &server.endpoint)
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

fn handle_fs_sync(args: FsSyncArgs, ui: Ui) -> Result<()> {
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
    ui.success("sync dry-run planned");
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
) -> Result<cocli_fs::SyncPlan> {
    let mut options = cocli_fs::ManifestOptions {
        include: include.to_vec(),
        ..cocli_fs::ManifestOptions::default()
    };
    if !exclude.is_empty() {
        options.exclude.extend(exclude.iter().cloned());
    }
    let manifest = cocli_fs::FileManifest::build(Path::new(local), &options)
        .map_err(|e| ColabError::config(e.to_string()))?;
    let remote = cocli_fs::FileManifest::default();
    Ok(cocli_fs::diff(&manifest, &remote, delete))
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

fn migration(ui: &Ui, old: &str, new: &str) {
    ui.info(&format!("old: {old}"));
    ui.info(&format!("new: {new}"));
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
    manifest: &cocli_protocol::ContinuationManifest,
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
) -> Result<cocli_protocol::ContinuationManifest> {
    let bytes = std::fs::read(continuation_path(config, name))?;
    cocli_protocol::ContinuationManifest::from_json(&bytes)
        .map_err(|e| ColabError::parse(e.to_string()))
}

fn git_snapshot() -> cocli_protocol::GitSnapshot {
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
    cocli_protocol::GitSnapshot {
        commit_hash,
        dirty_tree,
    }
}

fn append_audit(line: &str) -> Result<()> {
    let dir = cocli_core::data_dir().map_err(|e| ColabError::config(e.to_string()))?;
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

    let exit_code = shell::run_passthrough(client, &server, &command).await?;
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
    ccu: &Option<colab_cli::client::api::CcuInfo>,
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

    let result = shell::run_remote_tui(client, &server, &remote_cmd).await;

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
    let output = shell::capture_remote_command(client, server, cmd).await?;
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
    let refresher: shell::TokenRefresher = {
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

    shell::run_shell(client, &server, None, Some(refresher)).await
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

    let exit_code = shell::run_passthrough(client, &server, &argv).await?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn resolve_server<'a>(servers: &'a [StoredServer], name: Option<&str>) -> Result<&'a StoredServer> {
    match name {
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
}
