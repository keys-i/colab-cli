use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{CommandFactory, Parser};
use colored::Colorize;

use crate::cocli::auth;
use crate::cocli::cli::{
    AiCodeCommands, AiCommands, AiMcpCommands, AiToolsCommands, AuthCommands, AuthProfileArgs, Cli,
    Commands, CompatTransferArgs, ConfigCommands, ContinueCommands, DistributeCommands,
    DistributePoolCommands, DistributeRecipeCommands, DistributeRunArgs, DistributeShardCommands,
    EnvCommands, ExecCommands, FileCommands, FleetCommands, FleetConfigArgs, FsCommands,
    FsDiffArgs, FsDriveCommands, FsSyncArgs, JuliaCommands, JuliaPkgCommands, KernelActionArgs,
    KernelSessionArg, LogCommands, MountCommands, PipCommands, PkgCommands, RCommands,
    RPkgCommands, RenvCommands, RunCommands, RuntimeCommands, SecretCommands, ServerCommands,
    SessionCommands, SessionKernelCommands, SessionLogsArgs, SessionNameArg, SessionNewArgs,
    SettingsBillingCommands, SettingsCommands, SettingsExperimentsCommands, SettingsUiCommands,
    SettingsUpdateCommands, SkillCommands, SlurpCommands, StatusCommands, SupportCommands,
    ToolsCommands,
};
#[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
use crate::cocli::cli::{DevCommands, ReleaseCommands};
use crate::cocli::config::{self, ColabConfig};
use crate::cocli::debug::{self, Verbosity};
use crate::cocli::error::{ColabError, Result};
use crate::cocli::exec::runner;
use crate::cocli::kernel::{self, KernelInfoSummary, KernelLanguage};
use crate::cocli::secrets::{self, SecretBundle, SecretCliArgs};
use crate::cocli::session::ServerManager;
use crate::cocli::session::client::ColabClient;
use crate::cocli::session::model::{JupyterKernel, KernelSpecResponse, Session, Shape, Variant};
use crate::cocli::session::store::StoredServer;
use crate::cocli::ui::Ui;

pub async fn main_entry() {
    let _ = dotenvy::dotenv();

    if maybe_print_dynamic_run_help() {
        return;
    }

    let cli = Cli::parse();
    let stdout_tty = std::io::stdout().is_terminal();
    let stdin_tty = std::io::stdin().is_terminal();
    let ci = std::env::var_os("CI").is_some();
    let color_choice: config::ColorChoice = cli.color.parse().unwrap_or_default();
    let use_color = color_choice.enabled(
        cli.no_color || std::env::var_os("NO_COLOR").is_some(),
        ci || !stdout_tty,
        cli.quiet,
        cli.json,
    );
    colored::control::set_override(use_color);
    let verbosity = Verbosity::from_count(cli.verbose, cli.quiet);
    debug::set(verbosity);
    let ring_bell = config::terminal_bell_allowed(cli.bell, ci, cli.quiet || cli.json);
    let json_mode = cli.json;
    let verbose = debug::enabled(3);
    let interactive = interaction_allowed(&cli, stdout_tty, stdin_tty, ci) && !debug::enabled(1);
    let plain = cli.plain
        || ((ci || !stdout_tty) && color_choice != config::ColorChoice::Always)
        || !use_color;
    let ui = Ui::new(cli.quiet || cli.json, plain, interactive);

    if let Err(e) = run(cli, ui).await {
        if json_mode {
            print_error_json(&e);
        } else {
            print_human_error(&e, verbose, ui);
        }
        if ring_bell {
            eprint!("\x07");
        }

        if !json_mode {
            match &e {
                ColabError::NotAuthenticated => {
                    eprintln!("  Run `colab auth login` to sign in.");
                }
                ColabError::TooManyAssignments => {
                    eprintln!("  Run `colab session stop --name NAME` to remove one.");
                }
                _ => {}
            }
        }

        std::process::exit(1);
    }
}

fn maybe_print_dynamic_run_help() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let Some(pos) = args.iter().position(|arg| arg == "run") else {
        return false;
    };
    let after_run = &args[pos + 1..];
    if !after_run
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        return false;
    }
    if after_run
        .iter()
        .any(|arg| arg == "pip" || arg == "pkg" || arg == "julia" || arg == "r")
    {
        return false;
    }
    let language = cached_kernel_language().map(|info| info.language);
    print_run_help_for_language(language.as_ref());
    true
}

fn print_error_json(e: &ColabError) {
    let error = match e {
        ColabError::ApiError {
            status,
            url,
            body: _,
        } => {
            serde_json::json!({
                "kind": error_kind(e),
                "status": status,
                "reason": http_reason(*status),
                "operation": api_operation(url),
                "retryable": retryable_status(*status),
                "message": api_message(*status, url),
                "fix": api_fix(*status, url),
            })
        }
        ColabError::Drive(drive) => {
            serde_json::json!({
                "kind": drive.kind,
                "message": drive.message,
                "next_action": drive.next_action,
                "stage": drive.stage,
                "retryable": drive.retryable,
                "fix": drive.fixes,
            })
        }
        ColabError::Config(message) => serde_json::json!({
            "kind": error_kind(e),
            "message": message,
            "next_action": error_next_action(e),
        }),
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

fn print_human_error(e: &ColabError, verbose: bool, ui: Ui) {
    match e {
        ColabError::ApiError { status, url, body } => {
            ui.error(&format!("{} failed", api_operation(url)));
            eprintln!();
            eprintln!("Colab returned {} {}", status, http_reason(*status));
            if url.contains("/assign") && url.contains("shape=hm") {
                eprintln!("shape: High-RAM CPU");
            }
            eprintln!("retryable: {}", yes_no(retryable_status(*status)));
            eprintln!();
            eprintln!("{}", api_message(*status, url));
            if let Some(fix) = api_fix(*status, url) {
                eprintln!();
                eprintln!("try: {fix}");
            }
            if verbose {
                eprintln!();
                eprintln!("url: {}", debug::sanitize_url(url));
                if let Some(body) = body {
                    eprintln!("body: {}", trim_raw(&debug::redact(body)));
                }
            } else if body.is_some() {
                eprintln!();
                if debug::enabled(1) {
                    eprintln!("Use -vvv to see the sanitized server body");
                } else {
                    eprintln!("Use --verbose to see the server body");
                }
            }
        }
        ColabError::Drive(drive) => {
            eprintln!("Drive mount failed");
            eprintln!();
            eprintln!("{}", drive.message);
            if let Some(stage) = &drive.stage {
                eprintln!("stage: {stage}");
            }
            eprintln!("retryable: {}", yes_no(drive.retryable));
            if !drive.fixes.is_empty() {
                eprintln!();
                eprintln!("fix: {}", drive.fixes[0]);
                for fix in drive.fixes.iter().skip(1) {
                    eprintln!("     {fix}");
                }
            } else if let Some(next) = &drive.next_action {
                eprintln!();
                eprintln!("fix: {next}");
            }
            if verbose && let Some(raw) = &drive.raw {
                eprintln!("\n{}", trim_raw(&debug::redact(raw)));
            } else if drive.raw.is_some() {
                eprintln!();
                if debug::enabled(1) {
                    eprintln!("Use -vvv to see sanitized request details");
                } else {
                    eprintln!("Use --verbose to see the request details");
                }
            }
        }
        ColabError::Config(message) => eprintln!("{message}"),
        ColabError::Network(error) => {
            eprintln!("Network request failed");
            eprintln!();
            eprintln!("{}", network_error_message(error));
            eprintln!(
                "retryable: {}",
                yes_no(error.is_timeout() || error.is_connect())
            );
            if verbose {
                eprintln!();
                if let Some(url) = error.url() {
                    eprintln!("url: {}", debug::sanitize_url(url.as_str()));
                }
                eprintln!("source: {}", debug::redact(&error.to_string()));
            } else {
                eprintln!();
                if debug::enabled(1) {
                    eprintln!("Use -vvv for sanitized request details");
                } else {
                    eprintln!("Use -v for request stages or -vvv for sanitized request details");
                }
            }
        }
        _ => ui.error(&e.to_string()),
    }
}

fn network_error_message(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "The request timed out"
    } else if error.is_connect() {
        "The endpoint is not reachable"
    } else if error.is_status() {
        "The server returned an error status"
    } else {
        "The request could not be completed"
    }
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
        ColabError::Drive(_) => "drive_error",
        ColabError::Io(_) => "io_error",
        ColabError::Network(_) => "network_error",
        ColabError::Json(_) => "json_error",
        ColabError::TomlDe(_) | ColabError::TomlSer(_) => "toml_error",
        ColabError::OAuth(_) => "oauth_error",
    }
}

fn error_next_action(e: &ColabError) -> Option<&'static str> {
    match e {
        ColabError::NotAuthenticated => Some("colab auth login"),
        ColabError::TooManyAssignments => Some("colab session stop --name NAME"),
        _ => None,
    }
}

fn http_reason(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "HTTP error",
    }
}

fn retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

fn api_operation(url: &str) -> &'static str {
    if url.contains("/assign") {
        "runtime assignment"
    } else if url.contains("/drive") {
        "Drive request"
    } else {
        "Colab request"
    }
}

fn api_message(status: u16, url: &str) -> &'static str {
    if url.contains("/assign") && retryable_status(status) {
        "Colab may be busy or the selected shape may be temporarily unavailable"
    } else if retryable_status(status) {
        "The service may be temporarily unavailable"
    } else {
        "The request was rejected by Colab"
    }
}

fn api_fix(status: u16, url: &str) -> Option<&'static str> {
    if url.contains("/assign") && retryable_status(status) && url.contains("shape=hm") {
        Some("run again with Standard RAM: colab session new --shape standard")
    } else if url.contains("/assign") && retryable_status(status) {
        Some("try again in a minute")
    } else if status == 401 || status == 403 {
        Some("run colab auth login")
    } else {
        None
    }
}

fn trim_raw(body: &str) -> String {
    let body = body.replace("<redacted>", "__COLAB_CLI_REDACTED__");
    let clean = strip_html(&body)
        .replace("__COLAB_CLI_REDACTED__", "<redacted>")
        .replace('\n', " ");
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.len() > 600 {
        format!("{}...", &clean[..600])
    } else {
        clean
    }
}

fn strip_html(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_tag = false;
    for ch in body.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn interaction_allowed(cli: &Cli, stdout_tty: bool, stdin_tty: bool, ci: bool) -> bool {
    if cli.no_interactive
        || cli.json
        || cli.quiet
        || ci
        || !stdout_tty
        || !stdin_tty
        || std::env::var_os("COLAB_CLI_NO_INTERACTIVE").is_some()
        || std::env::var_os("COLAB_NO_INTERACTIVE").is_some()
    {
        return false;
    }
    let Ok(path) = config::config_path() else {
        return true;
    };
    config::CocliConfig::load(&path)
        .map(|cfg| cfg.ui.tui != "never")
        .unwrap_or(true)
}

fn handle_launcher(ui: Ui) -> Result<()> {
    let _ = ui;
    let mut cmd = Cli::command();
    cmd.print_help()?;
    println!();
    Ok(())
}

fn cached_kernel_language() -> Option<KernelInfoSummary> {
    let config = ColabConfig::load(true).ok()?;
    let manager = make_manager(&config).ok()?;
    let server = manager
        .list_local()
        .ok()?
        .into_iter()
        .max_by_key(|s| s.date_assigned)?;
    let language = server
        .kernel_language
        .as_deref()
        .map(KernelLanguage::detect)?;
    Some(KernelInfoSummary {
        language,
        version: server.kernel_language_version,
    })
}

fn print_run_help_for_language(language: Option<&KernelLanguage>) {
    println!("Run code and prepare runtimes");
    println!();
    println!("Usage: colab run <COMMAND>");
    println!();
    println!("Commands:");
    match language {
        Some(KernelLanguage::Python) => {
            println!("  py        Run Python code");
            println!("  script    Run a script");
            println!("  notebook  Run a notebook");
            println!("  repl      Open kernel REPL");
            println!("  shell     Open runtime shell");
            println!("  pkg       Package commands for the active kernel");
            println!("  pip       Python package tools");
            println!("  ast       Code outline and execution view");
        }
        Some(KernelLanguage::Julia) => {
            println!("  code      Run code in the active kernel");
            println!("  script    Run a script");
            println!("  notebook  Run a notebook");
            println!("  repl      Open kernel REPL");
            println!("  shell     Open runtime shell");
            println!("  pkg       Package commands for the active kernel");
            println!("  julia     Julia tools");
        }
        Some(KernelLanguage::R) => {
            println!("  code      Run code in the active kernel");
            println!("  script    Run a script");
            println!("  notebook  Run a notebook");
            println!("  repl      Open kernel REPL");
            println!("  shell     Open runtime shell");
            println!("  pkg       Package commands for the active kernel");
            println!("  r         R tools");
        }
        _ => {
            println!("  code      Run code in the active kernel");
            println!("  script    Run a script");
            println!("  notebook  Run a notebook");
            println!("  repl      Open kernel REPL");
            println!("  shell     Open runtime shell");
            println!("  pkg       Package commands, if supported");
            println!();
            println!("kernel tools adapt after `colab session kernel refresh`");
        }
    }
}

fn load_colab_config(quiet: bool) -> Result<ColabConfig> {
    let config_path = config::config_path()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_string());
    let config = ColabConfig::load(quiet)?;
    debug::debug1(format!("config loaded path={config_path}"));
    debug::debug2(format!(
        "config data_dir={} session_store={}",
        config.data_dir.display(),
        config.servers_file().display()
    ));
    Ok(config)
}

fn command_namespace(command: &Option<Commands>) -> &'static str {
    match command {
        None => "help",
        Some(Commands::Session { command }) => match command {
            Some(SessionCommands::New(_)) => "session.new",
            Some(SessionCommands::List) => "session.list",
            Some(SessionCommands::Refresh) => "session.refresh",
            Some(SessionCommands::Repair(_)) => "session.repair",
            Some(SessionCommands::Reconnect(_)) => "session.reconnect",
            Some(SessionCommands::Url { .. }) => "session.url",
            Some(SessionCommands::Stop(_)) => "session.stop",
            Some(SessionCommands::Logs(_)) => "session.logs",
            Some(SessionCommands::Kernel { .. }) => "session.kernel",
            Some(SessionCommands::Last) => "session.last",
            _ => "session",
        },
        Some(Commands::Run { command }) => match command {
            RunCommands::Code { .. } => "run.code",
            RunCommands::Py { .. } => "run.py",
            RunCommands::Script { .. } => "run.script",
            RunCommands::Notebook { .. } => "run.notebook",
            RunCommands::Shell { .. } => "run.shell",
            RunCommands::Repl { .. } => "run.repl",
            RunCommands::Pip { command } => match command {
                PipCommands::Install { .. } => "run.pip.install",
                PipCommands::Freeze { .. } => "run.pip.freeze",
                PipCommands::Restore { .. } => "run.pip.restore",
                PipCommands::Check { .. } => "run.pip.check",
                PipCommands::List { .. } => "run.pip.list",
                PipCommands::Tree { .. } => "run.pip.tree",
                PipCommands::Cache { .. } => "run.pip.cache",
            },
            RunCommands::Pkg { command } => match command {
                PkgCommands::Add { .. } => "run.pkg.add",
                PkgCommands::Remove { .. } => "run.pkg.remove",
                PkgCommands::List { .. } => "run.pkg.list",
                PkgCommands::Status { .. } => "run.pkg.status",
                PkgCommands::Update { .. } => "run.pkg.update",
                PkgCommands::Restore { .. } => "run.pkg.restore",
                PkgCommands::Check { .. } => "run.pkg.check",
            },
            RunCommands::Julia { .. } => "run.julia",
            RunCommands::R { .. } => "run.r",
            RunCommands::Ast { .. } => "run.ast",
            RunCommands::Watch { .. } => "run.watch",
            _ => "run",
        },
        Some(Commands::Fs { command }) => match command {
            FsCommands::Ls { .. } => "fs.ls",
            FsCommands::Upload { .. } => "fs.upload",
            FsCommands::Download { .. } => "fs.download",
            FsCommands::Push { .. } => "fs.push",
            FsCommands::Pull { .. } => "fs.pull",
            FsCommands::Rm { .. } => "fs.rm",
            FsCommands::Sync(_) => "fs.sync",
            FsCommands::Changed(_) => "fs.changed",
            FsCommands::Diff(_) => "fs.diff",
            FsCommands::Drive { command } => match command {
                FsDriveCommands::Mount { .. } => "fs.drive.mount",
                FsDriveCommands::Status { .. } => "fs.drive.status",
                FsDriveCommands::List { .. } => "fs.drive.list",
                FsDriveCommands::Unmount { .. } => "fs.drive.unmount",
                FsDriveCommands::Path { .. } => "fs.drive.path",
            },
            _ => "fs",
        },
        Some(Commands::Status { command }) => match command {
            Some(StatusCommands::Runtime { .. }) => "status.runtime",
            Some(StatusCommands::Auth) => "status.auth",
            Some(StatusCommands::Drive) => "status.drive",
            Some(StatusCommands::Fs) => "status.fs",
            Some(StatusCommands::Kernel { .. }) => "status.kernel",
            Some(StatusCommands::Check) => "status.check",
            Some(StatusCommands::Version) => "status.version",
            _ => "status",
        },
        Some(Commands::Log { command, .. }) => match command {
            Some(LogCommands::List { .. }) => "log.list",
            Some(LogCommands::Show { .. }) => "log.show",
            Some(LogCommands::Export { .. }) => "log.export",
            Some(LogCommands::Tail { .. }) => "log.tail",
            None => "log",
        },
        Some(Commands::Ai { command }) => match command {
            Some(AiCommands::Tools { .. }) => "ai.tools",
            Some(AiCommands::Mcp { .. }) => "ai.mcp",
            Some(AiCommands::Plan { .. }) => "ai.plan",
            Some(AiCommands::Audit { .. }) => "ai.audit",
            Some(AiCommands::Explain { .. }) => "ai.explain",
            Some(AiCommands::Run { .. }) => "ai.run",
            Some(AiCommands::Code { .. }) => "ai.code",
            _ => "ai",
        },
        Some(Commands::Secret { command }) => match command {
            SecretCommands::List => "secret.list",
            SecretCommands::Set { .. } => "secret.set",
            SecretCommands::Unset { .. } => "secret.unset",
            SecretCommands::Inject { .. } => "secret.inject",
            SecretCommands::Status => "secret.status",
            SecretCommands::Doctor => "secret.doctor",
            SecretCommands::ExportRedacted => "secret.export-redacted",
        },
        Some(Commands::Auth { command }) => match command {
            AuthCommands::Login { method } if method == "adc" => "auth.login.adc",
            AuthCommands::Login { .. } => "auth.login.oauth2",
            AuthCommands::List { .. } => "auth.list",
            AuthCommands::Status { .. } => "auth.status",
            AuthCommands::Use { .. } => "auth.use",
            AuthCommands::Logout { .. } => "auth.logout",
            AuthCommands::ExportRedacted { .. } => "auth.export-redacted",
            _ => "auth",
        },
        Some(Commands::Settings { command }) => match command {
            Some(SettingsCommands::Get { .. }) => "settings.get",
            Some(SettingsCommands::Set { .. }) => "settings.set",
            Some(SettingsCommands::Path) => "settings.path",
            Some(SettingsCommands::Edit) => "settings.edit",
            Some(SettingsCommands::Reset { .. }) => "settings.reset",
            Some(SettingsCommands::Ui { .. }) => "settings.ui",
            Some(SettingsCommands::Experiments { .. }) => "settings.experiments",
            Some(SettingsCommands::Support { .. }) => "settings.support",
            Some(SettingsCommands::Update { .. }) => "settings.update",
            Some(SettingsCommands::Billing { .. }) => "settings.billing",
            _ => "settings",
        },
        Some(Commands::Continue { .. }) => "continue",
        Some(Commands::Distribute { .. }) => "distribute",
        Some(Commands::Update { .. }) => "update",
        Some(Commands::Version) => "version",
        Some(Commands::Pay { .. }) => "pay",
        Some(Commands::Completions { .. }) => "completions",
        _ => "compat",
    }
}

async fn run(cli: Cli, ui: Ui) -> Result<()> {
    if let Some(Commands::Completions { shell }) = &cli.command {
        let mut cmd = Cli::command();
        clap_complete::generate(*shell, &mut cmd, "colab", &mut std::io::stdout());
        return Ok(());
    }

    let json = cli.json;
    let secret_args = SecretCliArgs {
        env: cli.secret_env.clone(),
        env_file: cli.secret_env_file.clone(),
        secret: cli.secret.clone(),
    };
    if !secret_args.is_empty() && !matches!(&cli.command, Some(Commands::Run { .. })) {
        return Err(ColabError::config(
            "secret injection flags only work with colab run",
        ));
    }
    debug::debug1(format!("command {}", command_namespace(&cli.command)));
    match cli.command {
        None => handle_launcher(ui),
        Some(Commands::Auth { command }) => handle_auth(command, ui, json).await,
        Some(Commands::Session { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_session(command, &config, ui, json).await
        }
        Some(Commands::Run { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_run_space(command, &config, ui, cli.json, secret_args).await
        }
        Some(Commands::Exec { command }) => {
            migration(&ui, "colab run ...");
            let config = load_colab_config(cli.quiet)?;
            let no_secrets = SecretBundle::default();
            handle_exec(command, &config, ui, &no_secrets).await
        }
        Some(Commands::Fs { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_fs(command, &config, ui, json).await
        }
        Some(Commands::Mount { command }) => {
            migration(&ui, mount_migration_target(&command));
            let config = load_colab_config(cli.quiet)?;
            handle_mount(command, &config, ui, json).await
        }
        Some(Commands::Env { command }) => {
            migration(&ui, "colab run pip install/freeze/restore");
            let config = load_colab_config(cli.quiet)?;
            let no_secrets = SecretBundle::default();
            handle_env(command, &config, ui, &no_secrets).await
        }
        Some(Commands::Runtime { command }) => {
            migration(&ui, runtime_migration_target(&command));
            let config = load_colab_config(cli.quiet)?;
            handle_runtime(command, &config, ui, json).await
        }
        Some(Commands::Status { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_status(command, &config, ui, json).await
        }
        Some(Commands::Log {
            command,
            session,
            tail,
            format,
            out,
        }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_log(
                command,
                &config,
                ui,
                json,
                LogDefaults {
                    session,
                    tail,
                    format,
                    out,
                },
            )
            .await
        }
        Some(Commands::Tools { command }) => {
            migration(&ui, "colab ai tools ...");
            handle_tools(command, ui, json)
        }
        Some(Commands::Fleet { command }) => {
            migration(&ui, "colab distribute pool ...");
            handle_fleet(command, ui, json)
        }
        Some(Commands::Distribute { command }) => handle_distribute(command, ui, json),
        Some(Commands::Ai { command }) => handle_ai(command, ui, json),
        Some(Commands::Secret { command }) => handle_secret(command, ui, json),
        Some(Commands::Slurp { command }) => {
            migration(&ui, "colab distribute recipe ...");
            require_experiment("distribute", |cfg| cfg.experiments.distribute)?;
            handle_slurp(command, ui, json)
        }
        Some(Commands::Agent { command }) => {
            migration(&ui, "colab ai ...");
            let _ = command;
            Err(ColabError::config("old agent command is disabled"))
        }
        Some(Commands::Continue { command }) => {
            require_experiment("continue", |cfg| cfg.experiments.continue_work)?;
            let config = load_colab_config(cli.quiet)?;
            handle_continue(command, &config, ui, json).await
        }
        Some(Commands::Settings { command }) => handle_settings(command, ui, json),
        Some(Commands::Config { command }) => {
            migration(&ui, config_migration_target(&command));
            handle_config(command, json)
        }
        Some(Commands::Doctor { .. }) => {
            migration(&ui, "colab status check");
            let config = load_colab_config(cli.quiet)?;
            handle_status(Some(StatusCommands::Check), &config, ui, json).await
        }
        Some(Commands::BugReport { show_private }) => handle_bug_report(show_private, json),
        Some(Commands::Server { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_server(command, &config, ui).await
        }
        Some(Commands::File { command }) => {
            let config = load_colab_config(cli.quiet)?;
            handle_file(command, &config, ui).await
        }
        Some(Commands::CompatNew(args)) => {
            migration(&ui, "colab session new");
            let config = load_colab_config(cli.quiet)?;
            handle_session(Some(SessionCommands::New(args)), &config, ui, json).await
        }
        Some(Commands::CompatSessions) => {
            migration(&ui, "colab session list");
            let config = load_colab_config(cli.quiet)?;
            handle_session(Some(SessionCommands::List), &config, ui, json).await
        }
        Some(Commands::CompatStop(arg)) => {
            migration(&ui, "colab session stop");
            let config = load_colab_config(cli.quiet)?;
            handle_session(Some(SessionCommands::Stop(arg)), &config, ui, json).await
        }
        Some(Commands::CompatUpload(args)) => {
            migration(&ui, "colab fs upload LOCAL REMOTE");
            let config = load_colab_config(cli.quiet)?;
            compat_transfer(args, true, &config, ui).await
        }
        Some(Commands::CompatDownload(args)) => {
            migration(&ui, "colab fs download REMOTE LOCAL");
            let config = load_colab_config(cli.quiet)?;
            compat_transfer(args, false, &config, ui).await
        }
        Some(Commands::Update { install, yes }) => handle_update(install, yes, json),
        Some(Commands::Version) => print_version_info(json),
        Some(Commands::Pay { dry_run }) => {
            handle_settings_billing(SettingsBillingCommands::Open { dry_run }, json)
        }
        Some(Commands::Completions { .. }) => unreachable!(),
    }
}

async fn handle_auth(cmd: AuthCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        AuthCommands::Login { method } if method == "adc" => handle_auth_adc_login(ui, json),
        AuthCommands::Login { .. } => {
            let config = load_colab_config(ui.quiet)?;
            handle_login(&config, ui).await
        }
        AuthCommands::Logout { profile: None } => {
            auth::logout()?;
            ui.success("Signed out. Credentials cleared.");
            Ok(())
        }
        AuthCommands::Logout {
            profile: Some(name),
        } => {
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
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
        AuthCommands::Add(args) => {
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
            handle_auth_add(args, ui)
        }
        AuthCommands::List { show_private } => {
            let store = load_auth_profiles().ok();
            let profiles: Vec<_> = store
                .as_ref()
                .map(|store| {
                    store
                        .profiles
                        .iter()
                        .map(|p| redacted_profile(p, show_private))
                        .collect()
                })
                .unwrap_or_default();
            let current = auth::current_account()?.map(|account| {
                crate::cocli::auth::redaction::redacted_email(&account.email, show_private)
            });
            print_value(
                json,
                &serde_json::json!({
                    "current": current,
                    "active": store.as_ref().and_then(|s| s.active.clone()),
                    "profiles": profiles
                }),
            )
        }
        AuthCommands::Status { name, show_private } => {
            if let Some(name) = name {
                require_experiment("multi-login", |cfg| {
                    cfg.experiments.distribute && cfg.experiments.multi_login
                })?;
                let store = load_auth_profiles()?;
                let profile = store
                    .get(&name)
                    .ok_or_else(|| ColabError::config(format!("auth profile not found: {name}")))?;
                let profile = redacted_profile(profile, show_private);
                if json {
                    print_value(true, &profile)
                } else {
                    print_auth_profile_status(&profile);
                    Ok(())
                }
            } else {
                let current = auth::current_account()?.map(|account| {
                    crate::cocli::auth::redaction::redacted_email(&account.email, show_private)
                });
                let adc_path = adc_credentials_path();
                let adc_available = adc_path.as_ref().is_some_and(|p| p.exists());
                let adc_path = adc_path.map(|p| p.display().to_string());
                let data = serde_json::json!({
                    "signed_in": current.is_some(),
                    "account": current,
                    "adc_available": adc_available,
                    "adc_path": adc_path,
                });
                if json {
                    print_value(true, &data)
                } else {
                    print_auth_status(data);
                    Ok(())
                }
            }
        }
        AuthCommands::Use {
            name,
            allow_fallback_account,
        } => {
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
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
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
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
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
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
            require_experiment("multi-login", |cfg| {
                cfg.experiments.distribute && cfg.experiments.multi_login
            })?;
            let store = load_auth_profiles()?;
            let profile = store
                .get(&name)
                .ok_or_else(|| ColabError::config(format!("auth profile not found: {name}")))?;
            let data = serde_json::json!({
                "name": profile.name,
                "kind": profile.kind.to_string(),
                "auto_fallback": false,
                "note": "colab never switches accounts automatically to work around limits"
            });
            print_value(json, &data)
        }
    }
}

fn handle_auth_adc_login(ui: Ui, json: bool) -> Result<()> {
    let path = adc_credentials_path();
    let available = path.as_ref().is_some_and(|path| path.exists());
    if json {
        return print_value(
            true,
            &serde_json::json!({
                "method": "adc",
                "available": available,
                "path": path.map(|p| p.display().to_string()),
                "setup": "gcloud auth application-default login"
            }),
        );
    }
    if available {
        ui.success("ADC credentials found");
        if let Some(path) = path {
            println!("path: {}", path.display());
        }
    } else {
        println!("ADC credentials missing");
        println!("fix: gcloud auth application-default login");
    }
    Ok(())
}

fn adc_credentials_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GOOGLE_APPLICATION_CREDENTIALS") {
        return Some(PathBuf::from(path));
    }
    dirs::home_dir().map(|home| home.join(".config/gcloud/application_default_credentials.json"))
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

async fn handle_session(
    cmd: Option<SessionCommands>,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match cmd {
        None => handle_session_menu(config, ui).await,
        Some(SessionCommands::New(args)) => {
            let (variant, accelerator) = session_accelerator(&args)?;
            let shape = shape_from_args(&args)?;
            let retries = session_retries(&args);
            handle_assign(
                config,
                ui,
                AssignOptions {
                    variant: Some(variant),
                    accelerator,
                    name: args.name,
                    shape,
                    keepalive: args.keepalive,
                    retries,
                },
            )
            .await
        }
        Some(SessionCommands::List) => handle_ls(config, ui).await,
        Some(SessionCommands::Status(SessionNameArg { session })) => {
            migration(&ui, "colab status session --name NAME");
            handle_info(config, ui, session).await
        }
        Some(SessionCommands::Stop(SessionNameArg { session })) => {
            handle_rm(config, ui, session).await
        }
        Some(SessionCommands::Url { session, open }) => handle_url(config, ui, session, open).await,
        Some(SessionCommands::Last) => {
            let manager = make_manager(config)?;
            let servers = manager.list_local()?;
            let last = servers
                .iter()
                .max_by_key(|s| s.date_assigned)
                .ok_or_else(|| {
                    ColabError::config("no active session - run `colab session list`")
                })?;
            ui.print_server_status(last);
            Ok(())
        }
        Some(SessionCommands::Refresh) => handle_session_refresh(config, ui).await,
        Some(SessionCommands::Repair(SessionNameArg { session })) => {
            handle_session_repair(config, ui, session).await
        }
        Some(SessionCommands::Reconnect(SessionNameArg { session })) => {
            handle_session_reconnect(config, ui, session).await
        }
        Some(SessionCommands::Logs(args)) => handle_session_logs(config, ui, args).await,
        Some(SessionCommands::Kernel { command }) => {
            handle_session_kernel(config, ui, json, command).await
        }
    }
}

async fn handle_session_refresh(config: &ColabConfig, ui: Ui) -> Result<()> {
    let manager = make_manager(config)?;
    match manager.list().await {
        Ok((servers, removed)) => {
            ui.success(&format!(
                "sessions refreshed: {} active, {} stale removed",
                servers.len(),
                removed
            ));
            Ok(())
        }
        Err(e) => Err(map_session_network_error("refresh_sessions", e)),
    }
}

async fn handle_session_repair(
    config: &ColabConfig,
    ui: Ui,
    session: Option<String>,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    validate_runtime_endpoint(server)?;
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        list_runtime_sessions(manager.client(), server),
    )
    .await
    {
        Ok(Ok(_)) => {
            ui.success(&format!("session looks reachable: {}", server.label));
            Ok(())
        }
        Ok(Err(e)) => Err(map_drive_stage_error("repair_session", server, e)),
        Err(_) => Err(drive_endpoint_error(
            "repair_session",
            server,
            "timeout",
            true,
            None,
        )),
    }
}

async fn handle_session_reconnect(
    config: &ColabConfig,
    ui: Ui,
    session: Option<String>,
) -> Result<()> {
    match handle_session_repair(config, ui, session).await {
        Ok(()) => Ok(()),
        Err(_) => Err(ColabError::config(
            "could not reconnect this local session to an active Colab runtime\nfix: colab session list --refresh\n     colab session new --name work",
        )),
    }
}

async fn handle_session_logs(config: &ColabConfig, ui: Ui, args: SessionLogsArgs) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, args.session.as_deref())?;
    let data = serde_json::json!({
        "session": server.label,
        "tail": args.tail,
        "format": args.format,
        "available": false,
        "logs": [],
        "note": "execution history is captured for commands run through colab; this session has no persisted log stream"
    });
    let body = match args.format.as_str() {
        "jsonl" => format!("{}\n", serde_json::to_string(&data)?),
        "md" => format!(
            "# Session logs\n\nsession: {}\n\nNo persisted log stream is available.\n",
            server.label
        ),
        "ipynb" => serde_json::json!({
            "nbformat": 4,
            "nbformat_minor": 5,
            "cells": [],
            "metadata": { "colab": data }
        })
        .to_string(),
        _ => format!(
            "Session logs\nsession: {}\n\nNo persisted log stream is available.\n",
            server.label
        ),
    };
    if let Some(out) = args.out {
        std::fs::write(&out, body)?;
        ui.success(&format!("logs written: {out}"));
    } else {
        print!("{body}");
    }
    Ok(())
}

struct LogDefaults {
    session: Option<String>,
    tail: usize,
    format: String,
    out: Option<String>,
}

async fn handle_log(
    command: Option<LogCommands>,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    defaults: LogDefaults,
) -> Result<()> {
    match command {
        None if defaults.session.is_some()
            || defaults.out.is_some()
            || defaults.format != "text"
            || defaults.tail != 50 =>
        {
            handle_session_logs(
                config,
                ui,
                SessionLogsArgs {
                    session: defaults.session,
                    tail: defaults.tail,
                    format: defaults.format,
                    out: defaults.out,
                },
            )
            .await
        }
        None | Some(LogCommands::List { session: None }) => {
            let manager = make_manager(config)?;
            let servers = manager.list_local().unwrap_or_default();
            if json {
                let rows: Vec<_> = servers
                    .iter()
                    .map(|server| {
                        serde_json::json!({
                            "session": server.label,
                            "available": false,
                            "note": "no persisted log stream"
                        })
                    })
                    .collect();
                return print_value(true, &rows);
            }
            println!("{}", heading("Log", ui));
            println!("Session history");
            println!();
            if servers.is_empty() {
                println!("No session history recorded yet.");
                return Ok(());
            }
            println!("{:<24} {:<10} Note", "Session", "Available");
            for server in servers {
                println!("{:<24} {:<10} no persisted log stream", server.label, "no");
            }
            Ok(())
        }
        Some(LogCommands::List {
            session: Some(session),
        }) => {
            handle_session_logs(
                config,
                ui,
                SessionLogsArgs {
                    session: Some(session),
                    tail: defaults.tail,
                    format: defaults.format,
                    out: defaults.out,
                },
            )
            .await
        }
        Some(LogCommands::Show { session, tail })
        | Some(LogCommands::Tail {
            session,
            lines: tail,
        }) => {
            handle_session_logs(
                config,
                ui,
                SessionLogsArgs {
                    session,
                    tail,
                    format: "text".to_string(),
                    out: None,
                },
            )
            .await
        }
        Some(LogCommands::Export {
            session,
            format,
            out,
        }) => {
            handle_session_logs(
                config,
                ui,
                SessionLogsArgs {
                    session,
                    tail: defaults.tail,
                    format,
                    out,
                },
            )
            .await
        }
    }
}

async fn handle_session_kernel(
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    command: SessionKernelCommands,
) -> Result<()> {
    match command {
        SessionKernelCommands::List(KernelSessionArg { session })
        | SessionKernelCommands::Status(KernelSessionArg { session }) => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let views = load_kernel_views(&manager, &server).await?;
            print_kernel_list(&views, json, ui)
        }
        SessionKernelCommands::Current(KernelSessionArg { session }) => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let view = current_kernel_view(&manager, &server).await?;
            cache_kernel_view(&manager, &server, &view)?;
            print_current_kernel(&view, json, ui)
        }
        SessionKernelCommands::Select { kernel, session } => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let views = load_kernel_views(&manager, &server).await?;
            let selected = if let Some(kernel) = kernel {
                views
                    .into_iter()
                    .find(|view| view.id == kernel || view.name == kernel)
                    .ok_or_else(|| ColabError::config(format!("kernel not found: {kernel}")))?
            } else if ui.interactive {
                pick_kernel(views, ui)?
            } else {
                return Err(ColabError::config(
                    "session kernel select needs a kernel id/name outside a TTY",
                ));
            };
            cache_kernel_view(&manager, &server, &selected)?;
            if json {
                print_value(true, &selected)
            } else {
                kernel_action_progress(
                    &ui,
                    "Kernel select",
                    &[("selected kernel", &selected.name)],
                );
                ui.success(&format!("selected kernel {}", selected.name));
                Ok(())
            }
        }
        SessionKernelCommands::Specs(KernelSessionArg { session }) => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let specs = manager
                .client()
                .list_kernelspecs(&server.proxy_url, &server.proxy_token)
                .await?;
            print_kernel_specs(&specs, json, ui)
        }
        SessionKernelCommands::Start { spec, session } => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            kernel_action_progress(&ui, "Kernel start", &[("kernelspec", &spec)]);
            let kernel = manager
                .client()
                .start_kernel(&server.proxy_url, &server.proxy_token, &spec)
                .await?;
            let session = session_for_kernel(kernel);
            let info = detect_kernel_info(manager.client(), &server, &session, None).await;
            let view = KernelView::from_session(&server, &session, info, true)?;
            cache_kernel_view(&manager, &server, &view)?;
            print_current_kernel(&view, json, ui)
        }
        SessionKernelCommands::Interrupt(KernelActionArgs { session, yes }) => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let view = current_kernel_view(&manager, &server).await?;
            if view.state == "busy" && !yes {
                if !ui.interactive {
                    return Err(ColabError::config(
                        "kernel interrupt requires --yes because the kernel is busy",
                    ));
                }
                if !confirm_kernel_action(ui, "Interrupt busy kernel?")? {
                    return Err(ColabError::config("kernel interrupt cancelled"));
                }
            }
            kernel_action_progress(
                &ui,
                "Kernel interrupt",
                &[("selected kernel", &view.name), ("sending interrupt", "")],
            );
            manager
                .client()
                .kernel_action(
                    &server.proxy_url,
                    &server.proxy_token,
                    &view.id,
                    "interrupt",
                )
                .await?;
            if json {
                print_value(
                    true,
                    &serde_json::json!({
                        "ok": true,
                        "kernel_id": view.id,
                        "action": "interrupt",
                        "language": view.language.as_config_value(),
                        "version": view.version
                    }),
                )
            } else {
                ui.success("interrupted");
                Ok(())
            }
        }
        SessionKernelCommands::Restart {
            session,
            yes,
            timeout,
        } => {
            if !yes {
                if !ui.interactive {
                    return Err(ColabError::config(
                        "kernel restart requires --yes; it loses in-kernel state",
                    ));
                }
                if !confirm_kernel_action(ui, "Restart kernel and lose in-kernel state?")? {
                    return Err(ColabError::config("kernel restart cancelled"));
                }
            }
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let view = current_kernel_view(&manager, &server).await?;
            kernel_action_progress(
                &ui,
                "Kernel restart",
                &[("selected kernel", &view.name), ("sending restart", "")],
            );
            manager
                .client()
                .kernel_action(&server.proxy_url, &server.proxy_token, &view.id, "restart")
                .await?;
            let refreshed = wait_for_kernel_ready(&manager, &server, &view.id, timeout).await?;
            cache_kernel_view(&manager, &server, &refreshed)?;
            if json {
                print_value(
                    true,
                    &serde_json::json!({
                        "ok": true,
                        "kernel_id": refreshed.id,
                        "action": "restart",
                        "language": refreshed.language.as_config_value(),
                        "version": refreshed.version
                    }),
                )
            } else {
                ui.success(&format!("kernel ready {}", refreshed.language_display()));
                Ok(())
            }
        }
        SessionKernelCommands::Shutdown { session, yes } => {
            if !yes {
                if !ui.interactive {
                    return Err(ColabError::config(
                        "kernel shutdown requires --yes; it may break the current session",
                    ));
                }
                if !confirm_kernel_action(
                    ui,
                    "Shutdown kernel? This may break the current session.",
                )? {
                    return Err(ColabError::config("kernel shutdown cancelled"));
                }
            }
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            let view = current_kernel_view(&manager, &server).await?;
            kernel_action_progress(
                &ui,
                "Kernel shutdown",
                &[("selected kernel", &view.name), ("sending shutdown", "")],
            );
            manager
                .client()
                .shutdown_kernel(&server.proxy_url, &server.proxy_token, &view.id)
                .await?;
            mark_kernel_cache_stale(&manager, &server)?;
            if json {
                print_value(
                    true,
                    &serde_json::json!({ "ok": true, "kernel_id": view.id, "action": "shutdown" }),
                )
            } else {
                ui.success("kernel shut down");
                Ok(())
            }
        }
        SessionKernelCommands::Refresh(KernelSessionArg { session }) => {
            let (manager, server) = kernel_server(config, ui, session.as_deref()).await?;
            kernel_action_progress(
                &ui,
                "Kernel refresh",
                &[("reading kernels", ""), ("reading kernel info", "")],
            );
            let view = current_kernel_view(&manager, &server).await?;
            cache_kernel_view(&manager, &server, &view)?;
            print_current_kernel(&view, json, ui)
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct KernelView {
    selected: bool,
    name: String,
    language: KernelLanguage,
    version: Option<String>,
    state: String,
    id: String,
    session_id: String,
}

impl KernelView {
    fn from_session(
        server: &StoredServer,
        session: &Session,
        info: KernelInfoSummary,
        fallback_selected: bool,
    ) -> Result<Self> {
        let kernel = session
            .kernel
            .as_ref()
            .ok_or_else(|| ColabError::config("kernel unavailable"))?;
        let selected = server
            .selected_kernel_id
            .as_deref()
            .map(|id| id == kernel.id)
            .unwrap_or(fallback_selected);
        Ok(Self {
            selected,
            name: kernel.name.clone().unwrap_or_else(|| "unknown".to_string()),
            language: info.language,
            version: info.version,
            state: kernel
                .execution_state
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            id: kernel.id.clone(),
            session_id: session.id.clone(),
        })
    }

    fn language_display(&self) -> String {
        KernelInfoSummary {
            language: self.language.clone(),
            version: self.version.clone(),
        }
        .display()
    }
}

async fn kernel_server(
    config: &ColabConfig,
    ui: Ui,
    session: Option<&str>,
) -> Result<(ServerManager, StoredServer)> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session)?;
    validate_runtime_endpoint(server)?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    Ok((manager, server))
}

async fn load_kernel_views(
    manager: &ServerManager,
    server: &StoredServer,
) -> Result<Vec<KernelView>> {
    let specs = manager
        .client()
        .list_kernelspecs(&server.proxy_url, &server.proxy_token)
        .await
        .ok();
    let mut sessions = list_runtime_sessions(manager.client(), server).await?;
    let api_kernels = manager
        .client()
        .list_kernels(&server.proxy_url, &server.proxy_token)
        .await
        .unwrap_or_default();
    for kernel in api_kernels {
        if !sessions
            .iter()
            .any(|session| session.kernel.as_ref().map(|k| &k.id) == Some(&kernel.id))
        {
            sessions.push(session_for_kernel(kernel));
        }
    }

    let mut views = Vec::new();
    for (idx, session) in sessions.iter().filter(|s| s.kernel.is_some()).enumerate() {
        let info = detect_kernel_info(manager.client(), server, session, specs.as_ref()).await;
        views.push(KernelView::from_session(server, session, info, idx == 0)?);
    }
    if views.is_empty() {
        return Err(ColabError::config(
            "no running kernels\nfix: colab session url --open",
        ));
    }
    if server.selected_kernel_id.is_some() && !views.iter().any(|view| view.selected) {
        if let Some(first) = views.first_mut() {
            first.selected = true;
        }
    }
    Ok(views)
}

async fn list_runtime_sessions(
    client: &ColabClient,
    server: &StoredServer,
) -> Result<Vec<Session>> {
    match client
        .list_sessions(&server.proxy_url, &server.proxy_token)
        .await
    {
        Ok(sessions) => Ok(sessions),
        Err(proxy_error) => {
            debug::debug1("runtime sessions proxy path failed; trying tunnel path");
            client
                .list_sessions_via_tunnel(&server.endpoint)
                .await
                .map_err(|_| proxy_error)
        }
    }
}

fn session_for_kernel(kernel: JupyterKernel) -> Session {
    Session {
        id: uuid::Uuid::new_v4().to_string(),
        kernel: Some(kernel),
    }
}

async fn detect_kernel_info(
    client: &ColabClient,
    server: &StoredServer,
    session: &Session,
    specs: Option<&KernelSpecResponse>,
) -> KernelInfoSummary {
    if let Ok(info) = runner::kernel_info(server, session, std::time::Duration::from_secs(8)).await
        && !matches!(info.language, KernelLanguage::Unknown(_))
    {
        return info;
    }
    let kernel_name = session
        .kernel
        .as_ref()
        .and_then(|kernel| kernel.name.as_deref());
    if let Some(spec) = kernel_name.and_then(|name| specs.and_then(|s| s.kernelspecs.get(name))) {
        return KernelInfoSummary::from_language_info(spec.spec.language.as_deref(), None);
    }
    let _ = client;
    KernelInfoSummary::from_language_info(kernel_name, None)
}

async fn current_kernel_view(manager: &ServerManager, server: &StoredServer) -> Result<KernelView> {
    let mut views = load_kernel_views(manager, server).await?;
    if let Some(pos) = views.iter().position(|view| view.selected) {
        Ok(views.remove(pos))
    } else {
        Ok(views.remove(0))
    }
}

fn cache_kernel_view(
    manager: &ServerManager,
    server: &StoredServer,
    view: &KernelView,
) -> Result<()> {
    let mut updated = server.clone();
    updated.selected_kernel_id = Some(view.id.clone());
    updated.selected_kernel_name = Some(view.name.clone());
    updated.kernel_language = Some(view.language.as_config_value());
    updated.kernel_language_version = view.version.clone();
    updated.kernel_cache_stale = false;
    manager.save_local(updated)
}

fn mark_kernel_cache_stale(manager: &ServerManager, server: &StoredServer) -> Result<()> {
    let mut updated = server.clone();
    updated.kernel_cache_stale = true;
    manager.save_local(updated)
}

fn print_kernel_list(views: &[KernelView], json: bool, ui: Ui) -> Result<()> {
    if json {
        return print_value(true, &views.to_vec());
    }
    println!("{}", heading("Kernels", ui));
    println!();
    let headers = color_headers(
        &[
            "Selected",
            "Name",
            "Language",
            "Version",
            "State",
            "Kernel ID",
        ],
        ui,
    );
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let rows: Vec<Vec<String>> = views
        .iter()
        .map(|view| {
            vec![
                if view.selected {
                    "●".to_string()
                } else {
                    String::new()
                },
                view.name.clone(),
                language_cell(&view.language, ui),
                view.version.clone().unwrap_or_else(|| "-".to_string()),
                state_cell(&view.state, ui),
                crate::cocli::ui::width::truncate_middle(&view.id, 10),
            ]
        })
        .collect();
    print!(
        "{}",
        crate::cocli::ui::table::render_table(
            &header_refs,
            &rows,
            crate::cocli::ui::width::terminal_width()
        )
    );
    Ok(())
}

fn print_current_kernel(view: &KernelView, json: bool, ui: Ui) -> Result<()> {
    if json {
        return print_value(true, view);
    }
    println!("{}", heading("Current kernel", ui));
    println!();
    println!("  Name       {}", view.name);
    println!("  Language   {}", view.language_display());
    println!("  State      {}", view.state);
    println!(
        "  Kernel ID  {}",
        crate::cocli::ui::width::truncate_middle(&view.id, 16)
    );
    Ok(())
}

fn print_kernel_specs(specs: &KernelSpecResponse, json: bool, ui: Ui) -> Result<()> {
    if json {
        return print_value(true, specs);
    }
    println!("{}", heading("Kernel specs", ui));
    println!();
    let headers = color_headers(&["Name", "Display", "Language", "Default"], ui);
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let rows: Vec<Vec<String>> = specs
        .kernelspecs
        .iter()
        .map(|(name, spec)| {
            vec![
                name.clone(),
                spec.spec
                    .display_name
                    .clone()
                    .unwrap_or_else(|| name.clone()),
                spec.spec
                    .language
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                yes_no(specs.default.as_deref() == Some(name)).to_string(),
            ]
        })
        .collect();
    print!(
        "{}",
        crate::cocli::ui::table::render_table(
            &header_refs,
            &rows,
            crate::cocli::ui::width::terminal_width()
        )
    );
    Ok(())
}

fn pick_kernel(views: Vec<KernelView>, ui: Ui) -> Result<KernelView> {
    print_kernel_list(&views, false, ui)?;
    println!();
    println!("↑/↓ move · enter select · esc back · q quit");
    let choices: Vec<String> = views
        .iter()
        .map(|view| format!("{}  {}  {}", view.name, view.language_display(), view.state))
        .collect();
    let default = views.iter().position(|view| view.selected).unwrap_or(0);
    let Some(choice) = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Select kernel")
        .items(&choices)
        .default(default)
        .interact_opt()
        .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?
    else {
        return Err(ColabError::config("kernel selection cancelled"));
    };
    Ok(views[choice].clone())
}

fn color_headers(headers: &[&str], ui: Ui) -> Vec<String> {
    headers
        .iter()
        .map(|header| {
            if ui.plain {
                (*header).to_string()
            } else {
                header.cyan().bold().to_string()
            }
        })
        .collect()
}

fn language_cell(language: &KernelLanguage, ui: Ui) -> String {
    let text = language.display_name();
    if ui.plain {
        return text;
    }
    match language {
        KernelLanguage::Python => text.cyan().to_string(),
        KernelLanguage::Julia => text.purple().to_string(),
        KernelLanguage::R => text.green().to_string(),
        _ => text.dimmed().to_string(),
    }
}

fn state_cell(state: &str, ui: Ui) -> String {
    if ui.plain {
        return state.to_string();
    }
    match state {
        "idle" => state.green().to_string(),
        "busy" => state.yellow().to_string(),
        "starting" => state.cyan().to_string(),
        "dead" => state.red().to_string(),
        _ => state.dimmed().to_string(),
    }
}

fn confirm_kernel_action(ui: Ui, prompt: &str) -> Result<bool> {
    if !ui.interactive {
        return Ok(false);
    }
    dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))
}

fn kernel_action_progress(ui: &Ui, title: &str, rows: &[(&str, &str)]) {
    if ui.quiet || debug::enabled(1) {
        return;
    }
    println!("{title}");
    println!("{}", rule(*ui));
    println!();
    for (idx, (label, detail)) in rows.iter().enumerate() {
        let mark = if idx == 0 { "✓" } else { "·" };
        println!("{mark} {label:<22} {detail}");
    }
}

async fn wait_for_kernel_ready(
    manager: &ServerManager,
    server: &StoredServer,
    kernel_id: &str,
    timeout_secs: u64,
) -> Result<KernelView> {
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs.max(1));
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(ColabError::config("kernel restart timed out"));
        }
        let views = tokio::time::timeout(
            remaining.min(std::time::Duration::from_secs(5)),
            load_kernel_views(manager, server),
        )
        .await;
        if let Ok(Ok(mut views)) = views
            && let Some(view) = views.iter_mut().find(|view| view.id == kernel_id)
        {
            view.selected = true;
            if view.state != "starting" {
                return Ok(view.clone());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

async fn handle_session_menu(config: &ColabConfig, ui: Ui) -> Result<()> {
    let actions = [
        ("New session", "Assign a Colab runtime"),
        ("List sessions", "Show local assigned runtimes"),
        ("Last session", "Inspect the latest runtime"),
        ("Open session URL", "Print the latest runtime URL"),
        ("Stop session", "Stop the latest runtime"),
        ("Close", "Return to shell"),
    ];
    if ui.interactive {
        let choice = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Session")
            .items(&actions.map(|(name, note)| format!("{name} - {note}")))
            .default(0)
            .interact_opt()
            .map_err(|e| ColabError::config(format!("prompt cancelled: {e}")))?;
        return match choice {
            Some(0) => {
                handle_assign(
                    config,
                    ui,
                    AssignOptions {
                        variant: None,
                        accelerator: None,
                        name: None,
                        shape: Shape::Standard,
                        keepalive: false,
                        retries: 3,
                    },
                )
                .await
            }
            Some(1) => handle_ls(config, ui).await,
            Some(2) => {
                let manager = make_manager(config)?;
                let servers = manager.list_local()?;
                let last = servers
                    .iter()
                    .max_by_key(|s| s.date_assigned)
                    .ok_or_else(|| {
                        ColabError::config("no active session - run `colab session list`")
                    })?;
                ui.print_server_status(last);
                Ok(())
            }
            Some(3) => handle_url(config, ui, None, false).await,
            Some(4) => handle_rm(config, ui, None).await,
            _ => Ok(()),
        };
    }

    println!("Session");
    println!("Manage Colab sessions");
    println!();
    for (name, note) in actions.iter().take(5) {
        println!("  {:<17} {}", name, note);
    }
    Ok(())
}

async fn handle_run_space(
    cmd: RunCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    secret_args: SecretCliArgs,
) -> Result<()> {
    let secret_bundle = resolve_secret_bundle(&secret_args, ui, json)?;
    match cmd {
        RunCommands::Code { session, code } => {
            handle_run_code(config, ui, session, code, json, &secret_bundle).await
        }
        RunCommands::Script {
            script,
            session,
            ast,
            args,
        } => {
            if ast {
                require_experiment("ast observer", |cfg| cfg.experiments.ast_observer)?;
                print_code_outline(&script, false)?;
            }
            handle_exec(
                ExecCommands::Run {
                    script,
                    session,
                    args,
                },
                config,
                ui,
                &secret_bundle,
            )
            .await
        }
        RunCommands::Py { session, code } => {
            handle_exec(
                ExecCommands::Py { session, code },
                config,
                ui,
                &secret_bundle,
            )
            .await
        }
        RunCommands::Notebook {
            notebook,
            session,
            out,
            ast,
        } => {
            if ast {
                require_experiment("ast observer", |cfg| cfg.experiments.ast_observer)?;
                print_code_outline(&notebook, false)?;
            }
            handle_exec(
                ExecCommands::Nb {
                    notebook,
                    session,
                    out,
                },
                config,
                ui,
                &secret_bundle,
            )
            .await
        }
        RunCommands::Repl { session } => {
            handle_repl(config, ui, session, json, &secret_bundle).await
        }
        RunCommands::Shell { session } => {
            handle_exec(ExecCommands::Shell { session }, config, ui, &secret_bundle).await
        }
        RunCommands::Pip { command } => handle_run_pip(command, config, ui, &secret_bundle).await,
        RunCommands::Pkg { command } => {
            handle_run_pkg(command, config, ui, json, &secret_bundle).await
        }
        RunCommands::Julia { command } => {
            handle_run_julia(command, config, ui, json, &secret_bundle).await
        }
        RunCommands::R { command } => handle_run_r(command, config, ui, json, &secret_bundle).await,
        RunCommands::Ast { file, json } => {
            require_experiment("AST observer", |cfg| cfg.experiments.ast_observer)?;
            print_code_outline(&file, json)
        }
        RunCommands::Watch {
            script,
            session,
            ast,
            args,
        } => {
            if ast {
                require_experiment("AST observer", |cfg| cfg.experiments.ast_observer)?;
                print_code_outline(&script, false)?;
            }
            handle_exec(
                ExecCommands::Run {
                    script,
                    session,
                    args,
                },
                config,
                ui,
                &secret_bundle,
            )
            .await
        }
        RunCommands::Install {
            packages,
            requirements,
            session,
        } => {
            migration(&ui, "colab run pip install ...");
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
                    &secret_bundle,
                )
                .await
            } else {
                handle_env(
                    EnvCommands::Install { packages, session },
                    config,
                    ui,
                    &secret_bundle,
                )
                .await
            }
        }
        RunCommands::Freeze { session } => {
            migration(&ui, "colab run pip freeze");
            handle_env(EnvCommands::Freeze { session }, config, ui, &secret_bundle).await
        }
        RunCommands::Restore {
            requirements,
            session,
        } => {
            migration(&ui, "colab run pip restore requirements.txt");
            handle_env(
                EnvCommands::Restore {
                    requirements,
                    session,
                },
                config,
                ui,
                &secret_bundle,
            )
            .await
        }
        RunCommands::Last { confirm } => {
            handle_exec(ExecCommands::Last { confirm }, config, ui, &secret_bundle).await
        }
        RunCommands::History => Err(ColabError::config(
            "run history has no command store yet - rerun commands explicitly",
        )),
    }
}

async fn handle_run_code(
    config: &ColabConfig,
    ui: Ui,
    session: Option<String>,
    code: String,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let (_view, kernel_session) = active_kernel_session(&manager, &server).await?;
    let output = runner::execute_colab_cell_in_session_with_secrets(
        manager.client(),
        &server,
        &kernel_session,
        &code,
        std::time::Duration::from_secs(60),
        secrets,
    )
    .await?;
    print_repl_output(&output, json)
}

fn resolve_secret_bundle(args: &SecretCliArgs, ui: Ui, json: bool) -> Result<SecretBundle> {
    if args.is_empty() {
        return Ok(SecretBundle::default());
    }
    require_experiment("secrets bridge", |cfg| cfg.experiments.secrets_bridge)?;
    let cfg = load_cocli_config()?;
    if !args.env.is_empty() && !cfg.secrets.allow_env {
        return Err(ColabError::config(
            "secret env injection is disabled in settings",
        ));
    }
    if !args.env_file.is_empty() && !cfg.secrets.allow_env_file {
        return Err(ColabError::config(
            "secret env-file injection is disabled in settings",
        ));
    }
    let bundle = secrets::resolve_from_process_env(args)?;
    if !bundle.is_empty() && !json && !ui.quiet {
        println!("Secrets");
        println!("{}", rule(ui));
        println!();
        for (key, source) in bundle.rows() {
            println!("✓ {:<14} from {source}", key);
        }
        println!("· bridge          userdata.get enabled");
        println!();
        if !ui.quiet && args.env.iter().any(|spec| spec.contains('=')) {
            eprintln!(
                "warning: passing secrets as CLI arguments can leak through shell history; prefer --env KEY or --prompt"
            );
        }
    }
    Ok(bundle)
}

async fn handle_run_pip(
    cmd: PipCommands,
    config: &ColabConfig,
    ui: Ui,
    secrets: &SecretBundle,
) -> Result<()> {
    let session_hint = match &cmd {
        PipCommands::Install { session, .. }
        | PipCommands::Freeze { session }
        | PipCommands::Restore { session, .. }
        | PipCommands::Check { session }
        | PipCommands::List { session }
        | PipCommands::Tree { session }
        | PipCommands::Cache { session } => session.as_deref(),
    };
    ensure_cached_language_allows(config, session_hint, KernelLanguage::Python)?;
    match cmd {
        PipCommands::Install {
            packages,
            requirements,
            session,
        } => {
            if let Some(requirements) = requirements {
                if !packages.is_empty() {
                    return Err(ColabError::config(
                        "run pip install accepts packages or -r requirements.txt, not both",
                    ));
                }
                handle_env(
                    EnvCommands::Restore {
                        requirements,
                        session,
                    },
                    config,
                    ui,
                    secrets,
                )
                .await
            } else {
                handle_env(
                    EnvCommands::Install { packages, session },
                    config,
                    ui,
                    secrets,
                )
                .await
            }
        }
        PipCommands::Freeze { session } => {
            handle_env(EnvCommands::Freeze { session }, config, ui, secrets).await
        }
        PipCommands::Restore {
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
                secrets,
            )
            .await
        }
        PipCommands::Check { session } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-m".into(), "pip".into(), "check".into()],
                secrets,
            )
            .await
        }
        PipCommands::List { session } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-m".into(), "pip".into(), "list".into()],
                secrets,
            )
            .await
        }
        PipCommands::Tree { session } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-m".into(), "pip".into(), "list".into()],
                secrets,
            )
            .await
        }
        PipCommands::Cache { session } => {
            handle_run(
                config,
                ui,
                session,
                vec![
                    "python".into(),
                    "-m".into(),
                    "pip".into(),
                    "cache".into(),
                    "info".into(),
                ],
                secrets,
            )
            .await
        }
    }
}

async fn handle_run_pkg(
    cmd: PkgCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    match cmd {
        PkgCommands::Add { packages, session } => {
            run_pkg_action(config, ui, session, "add", packages, json, secrets).await
        }
        PkgCommands::Remove { packages, session } => {
            run_pkg_action(config, ui, session, "remove", packages, json, secrets).await
        }
        PkgCommands::List { session } => {
            run_pkg_action(config, ui, session, "list", Vec::new(), json, secrets).await
        }
        PkgCommands::Status { session } => {
            run_pkg_action(config, ui, session, "status", Vec::new(), json, secrets).await
        }
        PkgCommands::Update { packages, session } => {
            run_pkg_action(config, ui, session, "update", packages, json, secrets).await
        }
        PkgCommands::Restore { file, session } => {
            run_pkg_action(
                config,
                ui,
                session,
                "restore",
                file.into_iter().collect(),
                json,
                secrets,
            )
            .await
        }
        PkgCommands::Check { session } => {
            run_pkg_action(config, ui, session, "check", Vec::new(), json, secrets).await
        }
    }
}

async fn handle_run_julia(
    cmd: JuliaCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    let JuliaCommands::Pkg { command } = cmd;
    match command {
        JuliaPkgCommands::Add { packages, session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "add", packages, json, secrets).await
        }
        JuliaPkgCommands::Status { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "status", Vec::new(), json, secrets).await
        }
        JuliaPkgCommands::Instantiate { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "restore", Vec::new(), json, secrets).await
        }
        JuliaPkgCommands::Precompile { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "precompile", Vec::new(), json, secrets).await
        }
        JuliaPkgCommands::Update { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "update", Vec::new(), json, secrets).await
        }
        JuliaPkgCommands::Test { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "test", Vec::new(), json, secrets).await
        }
        JuliaPkgCommands::Rm { packages, session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::Julia)?;
            run_pkg_action(config, ui, session, "remove", packages, json, secrets).await
        }
    }
}

async fn handle_run_r(
    cmd: RCommands,
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    match cmd {
        RCommands::Pkg { command } => match command {
            RPkgCommands::Install { packages, session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "add", packages, json, secrets).await
            }
            RPkgCommands::List { session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "list", Vec::new(), json, secrets).await
            }
            RPkgCommands::Update { session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "update", Vec::new(), json, secrets).await
            }
            RPkgCommands::Remove { packages, session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "remove", packages, json, secrets).await
            }
        },
        RCommands::Renv { command } => match command {
            RenvCommands::Restore { session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "restore", Vec::new(), json, secrets).await
            }
            RenvCommands::Snapshot { session } => {
                ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
                run_pkg_action(config, ui, session, "snapshot", Vec::new(), json, secrets).await
            }
        },
        RCommands::SessionInfo { session } => {
            ensure_cached_language_allows(config, session.as_deref(), KernelLanguage::R)?;
            run_pkg_action(config, ui, session, "status", Vec::new(), json, secrets).await
        }
    }
}

async fn run_pkg_action(
    config: &ColabConfig,
    ui: Ui,
    session: Option<String>,
    action: &str,
    args: Vec<String>,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let (view, kernel_session) = active_kernel_session(&manager, &server).await?;
    let Some(code) = kernel::package_code(&view.language, action, &args) else {
        return Err(ColabError::config(format!(
            "package tooling is not available for this kernel\nlanguage: {}\nfix: use `colab run code --code \"...\"`",
            view.language.display_name()
        )));
    };
    package_progress(&ui, action, &view, &args);
    let output = runner::execute_colab_cell_in_session_with_secrets(
        manager.client(),
        &server,
        &kernel_session,
        &code,
        std::time::Duration::from_secs(600),
        secrets,
    )
    .await?;
    print_repl_output(&output, json)
}

async fn active_kernel_session(
    manager: &ServerManager,
    server: &StoredServer,
) -> Result<(KernelView, Session)> {
    let mut sessions = list_runtime_sessions(manager.client(), server).await?;
    if sessions.is_empty() {
        let kernels = manager
            .client()
            .list_kernels(&server.proxy_url, &server.proxy_token)
            .await?;
        sessions.extend(kernels.into_iter().map(session_for_kernel));
    }
    let selected_pos = server
        .selected_kernel_id
        .as_ref()
        .and_then(|id| {
            sessions
                .iter()
                .position(|s| s.kernel.as_ref().map(|k| &k.id) == Some(id))
        })
        .unwrap_or(0);
    let session = sessions
        .get(selected_pos)
        .cloned()
        .ok_or_else(|| ColabError::config("no running kernels\nfix: colab session url --open"))?;
    let info = detect_kernel_info(manager.client(), server, &session, None).await;
    let view = KernelView::from_session(server, &session, info, true)?;
    cache_kernel_view(manager, server, &view)?;
    Ok((view, session))
}

fn ensure_cached_language_allows(
    config: &ColabConfig,
    session: Option<&str>,
    expected: KernelLanguage,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session)?;
    let Some(language) = server
        .kernel_language
        .as_deref()
        .map(KernelLanguage::detect)
    else {
        return Ok(());
    };
    if std::mem::discriminant(&language) == std::mem::discriminant(&expected) {
        return Ok(());
    }
    let tool = match expected {
        KernelLanguage::Python => "pip is Python tooling",
        KernelLanguage::Julia => "Julia package tooling",
        KernelLanguage::R => "R package tooling",
        _ => "package tooling",
    };
    Err(ColabError::config(format!(
        "{tool}, but the active kernel is {}\nuse: colab run pkg add <package>",
        language.display_name()
    )))
}

fn package_progress(ui: &Ui, action: &str, view: &KernelView, args: &[String]) {
    if ui.quiet || debug::enabled(1) {
        return;
    }
    println!("Package {action}");
    println!("{}", rule(*ui));
    println!();
    println!("✓ kernel          {}", view.language_display());
    let detail = if args.is_empty() {
        action.to_string()
    } else {
        args.join(" ")
    };
    match view.language {
        KernelLanguage::Python => println!("· pip {action:<10} {detail}"),
        KernelLanguage::Julia => println!("· Pkg.{action:<10} {detail}"),
        KernelLanguage::R => println!("· R packages      {detail}"),
        _ => println!("· package action  {detail}"),
    }
}

async fn handle_exec(
    cmd: ExecCommands,
    config: &ColabConfig,
    ui: Ui,
    secrets: &SecretBundle,
) -> Result<()> {
    match cmd {
        ExecCommands::Run {
            script,
            session,
            args,
        } => {
            let mut command = vec!["python".to_string(), script];
            command.extend(args);
            let command = apply_python_script_bridge(command, secrets)?;
            handle_run(config, ui, session, command, secrets).await
        }
        ExecCommands::Py { session, code } => {
            let code = apply_python_code_bridge(code, secrets)?;
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-c".into(), code],
                secrets,
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
            handle_run(config, ui, session, command, secrets).await
        }
        ExecCommands::Repl { session } => handle_repl(config, ui, session, false, secrets).await,
        ExecCommands::Shell { session } => handle_shell(config, ui, session, secrets).await,
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

fn apply_python_code_bridge(code: String, secrets: &SecretBundle) -> Result<String> {
    if secrets.is_empty() {
        return Ok(code);
    }
    Ok(format!("{}\n{code}", secrets.python_prelude()))
}

fn apply_python_script_bridge(command: Vec<String>, secrets: &SecretBundle) -> Result<Vec<String>> {
    if secrets.is_empty()
        || command.len() < 2
        || command.first().map(String::as_str) != Some("python")
    {
        return Ok(command);
    }
    let script = command[1].clone();
    let argv_json = serde_json::to_string(&command[1..])?;
    let script_json = serde_json::to_string(&script)?;
    let wrapper = format!(
        "{}\nimport runpy as _colab_cli_runpy, sys as _colab_cli_sys\n_colab_cli_sys.argv = {argv_json}\n_colab_cli_runpy.run_path({script_json}, run_name='__main__')",
        secrets.python_prelude()
    );
    Ok(vec!["python".into(), "-c".into(), wrapper])
}

async fn handle_repl(
    config: &ColabConfig,
    ui: Ui,
    session_name: Option<String>,
    json: bool,
    secrets: &SecretBundle,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, session_name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    debug::debug1("run.repl stage=check_jupyter_sessions attempt=1/1");
    let sessions = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        list_runtime_sessions(manager.client(), &server),
    )
    .await
    {
        Ok(Ok(sessions)) => sessions,
        Ok(Err(e)) => return Err(repl_runtime_error(&server, "check_jupyter_sessions", e)),
        Err(_) => {
            return Err(repl_endpoint_error(
                &server,
                "check_jupyter_sessions",
                "Runtime endpoint timed out",
            ));
        }
    };
    let session = sessions
        .iter()
        .find(|s| s.kernel.is_some())
        .ok_or_else(|| {
            ColabError::config("REPL needs a Colab kernel session\nfix: colab session url --open")
        })?;
    let kernel_id = session
        .kernel
        .as_ref()
        .map(|kernel| kernel.id.clone())
        .ok_or_else(|| ColabError::config("kernel unavailable"))?;
    let info = detect_kernel_info(manager.client(), &server, session, None).await;

    if !std::io::stdin().is_terminal() {
        let mut code = String::new();
        std::io::stdin().read_to_string(&mut code)?;
        if code.trim().is_empty() {
            return Ok(());
        }
        let output = execute_repl_code(
            manager.client(),
            &server,
            session,
            &kernel_id,
            &code,
            secrets,
        )
        .await?;
        return print_repl_output(&output, json);
    }

    if json {
        return Err(ColabError::config(
            "run repl --json reads code from stdin; interactive JSON REPL is not supported",
        ));
    }

    println!("Connected to {} · {}", server.label, info.display());

    let mut editor = ReplLineEditor::new(info.language.clone());
    loop {
        match editor.read_entry()? {
            ReplInput::Code(code) => {
                if repl_quit_command(&code) {
                    break;
                }
                let output = execute_repl_code(
                    manager.client(),
                    &server,
                    session,
                    &kernel_id,
                    &code,
                    secrets,
                )
                .await?;
                print_repl_output(&output, false)?;
            }
            ReplInput::Interrupt => {
                let _ = manager
                    .client()
                    .kernel_action(
                        &server.proxy_url,
                        &server.proxy_token,
                        &kernel_id,
                        "interrupt",
                    )
                    .await;
                println!("interrupted");
            }
            ReplInput::Eof => break,
        }
    }
    Ok(())
}

fn repl_runtime_error(server: &StoredServer, stage: &str, error: ColabError) -> ColabError {
    match error {
        ColabError::Network(e) => repl_endpoint_error(server, stage, network_error_message(&e)),
        ColabError::ApiError { status, .. } => repl_endpoint_error(
            server,
            stage,
            &format!("Colab returned {status} {}", http_reason(status)),
        ),
        ColabError::ServerNotFound { .. } => {
            repl_endpoint_error(server, stage, "Runtime endpoint is stale")
        }
        other => other,
    }
}

fn repl_endpoint_error(server: &StoredServer, stage: &str, message: &str) -> ColabError {
    ColabError::config(format!(
        "REPL failed\n\n{message}\nsession: {}\nstage: {stage}\n\nfix: colab session list --refresh\n     colab session new --name work",
        server.label
    ))
}

async fn execute_repl_code(
    client: &ColabClient,
    server: &StoredServer,
    session: &Session,
    kernel_id: &str,
    code: &str,
    secrets: &SecretBundle,
) -> Result<runner::CellOutput> {
    let execute = runner::execute_colab_cell_in_session_with_secrets(
        client,
        server,
        session,
        code,
        std::time::Duration::from_secs(30),
        secrets,
    );
    tokio::pin!(execute);
    tokio::select! {
        result = &mut execute => result,
        _ = tokio::signal::ctrl_c() => {
            client.kernel_action(&server.proxy_url, &server.proxy_token, kernel_id, "interrupt").await?;
            println!("interrupted");
            Ok(runner::CellOutput::default())
        }
    }
}

fn print_repl_output(output: &runner::CellOutput, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(output)?);
        return Ok(());
    }
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    if let Some(name) = &output.error_name {
        eprintln!("{name}: {}", output.error_value.as_deref().unwrap_or(""));
    }
    for line in &output.traceback {
        eprintln!("{line}");
    }
    Ok(())
}

fn repl_quit_command(code: &str) -> bool {
    matches!(code.trim(), "exit()" | "quit()" | "/quit")
}

#[derive(Debug, PartialEq, Eq)]
enum ReplInput {
    Code(String),
    Interrupt,
    Eof,
}

#[derive(Default)]
struct ReplLineEditor {
    history: Vec<String>,
    language: Option<KernelLanguage>,
}

impl ReplLineEditor {
    fn new(language: KernelLanguage) -> Self {
        Self {
            history: Vec::new(),
            language: Some(language),
        }
    }

    fn read_entry(&mut self) -> Result<ReplInput> {
        let mut lines = Vec::new();
        loop {
            let language = self.language.clone().unwrap_or(KernelLanguage::Python);
            let prompt = if lines.is_empty() {
                language.repl_prompt()
            } else {
                language.continuation_prompt()
            };
            match self.read_line(prompt)? {
                ReplInput::Code(line) => {
                    lines.push(line);
                    if matches!(language, KernelLanguage::Python) && python_needs_more_input(&lines)
                    {
                        continue;
                    }
                    let code = lines.join("\n");
                    if !code.trim().is_empty() {
                        self.history.push(code.clone());
                    }
                    return Ok(ReplInput::Code(code));
                }
                other => return Ok(other),
            }
        }
    }

    fn read_line(&mut self, prompt: &str) -> Result<ReplInput> {
        use crossterm::{
            cursor,
            event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
            execute,
            style::Print,
            terminal::{self, ClearType},
        };

        terminal::enable_raw_mode().map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
        let _raw = LocalRawModeGuard;

        let mut stdout = std::io::stdout();
        let mut chars: Vec<char> = Vec::new();
        let mut cursor_pos = 0usize;
        let mut history_pos: Option<usize> = None;
        render_repl_line(prompt, &chars, cursor_pos)?;

        loop {
            let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read().map_err(|e| ColabError::config(format!("terminal read: {e}")))?
            else {
                continue;
            };
            match (code, modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    stdout.write_all(b"\r\n")?;
                    stdout.flush()?;
                    return Ok(ReplInput::Interrupt);
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) if chars.is_empty() => {
                    stdout.write_all(b"\r\n")?;
                    stdout.flush()?;
                    return Ok(ReplInput::Eof);
                }
                (KeyCode::Enter, _) => {
                    stdout.write_all(b"\r\n")?;
                    stdout.flush()?;
                    return Ok(ReplInput::Code(chars.iter().collect()));
                }
                (KeyCode::Backspace, _) if cursor_pos > 0 => {
                    cursor_pos -= 1;
                    chars.remove(cursor_pos);
                }
                (KeyCode::Delete, _) if cursor_pos < chars.len() => {
                    chars.remove(cursor_pos);
                }
                (KeyCode::Left, _) if cursor_pos > 0 => cursor_pos -= 1,
                (KeyCode::Right, _) if cursor_pos < chars.len() => cursor_pos += 1,
                (KeyCode::Home, _) => cursor_pos = 0,
                (KeyCode::End, _) => cursor_pos = chars.len(),
                (KeyCode::Up, _) if !self.history.is_empty() => {
                    let next = history_pos
                        .map(|pos| pos.saturating_sub(1))
                        .unwrap_or_else(|| self.history.len() - 1);
                    history_pos = Some(next);
                    chars = self.history[next].chars().collect();
                    cursor_pos = chars.len();
                }
                (KeyCode::Down, _) if !self.history.is_empty() => {
                    if let Some(pos) = history_pos {
                        if pos + 1 < self.history.len() {
                            let next = pos + 1;
                            history_pos = Some(next);
                            chars = self.history[next].chars().collect();
                            cursor_pos = chars.len();
                        } else {
                            history_pos = None;
                            chars.clear();
                            cursor_pos = 0;
                        }
                    }
                }
                (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    chars.insert(cursor_pos, ch);
                    cursor_pos += 1;
                }
                _ => {}
            }
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                terminal::Clear(ClearType::CurrentLine),
                Print(prompt),
                Print(chars.iter().collect::<String>()),
                cursor::MoveToColumn((prompt.chars().count() + cursor_pos) as u16)
            )
            .map_err(|e| ColabError::config(format!("terminal render: {e}")))?;
            stdout.flush()?;
        }
    }
}

fn render_repl_line(prompt: &str, chars: &[char], cursor_pos: usize) -> Result<()> {
    use crossterm::{cursor, execute, style::Print, terminal};
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        terminal::Clear(terminal::ClearType::CurrentLine),
        Print(prompt),
        Print(chars.iter().collect::<String>()),
        cursor::MoveToColumn((prompt.chars().count() + cursor_pos) as u16)
    )
    .map_err(|e| ColabError::config(format!("terminal render: {e}")))?;
    stdout.flush()?;
    Ok(())
}

struct LocalRawModeGuard;

impl Drop for LocalRawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn python_needs_more_input(lines: &[String]) -> bool {
    let Some(last) = lines.last() else {
        return false;
    };
    let trimmed = last.trim_end();
    if lines.len() > 1 && trimmed.is_empty() {
        return false;
    }
    trimmed.ends_with('\\')
        || trimmed.ends_with(':')
        || bracket_balance(&lines.join("\n")) > 0
        || (lines.len() > 1 && !trimmed.is_empty())
}

fn bracket_balance(code: &str) -> i32 {
    let mut balance = 0;
    for ch in code.chars() {
        match ch {
            '(' | '[' | '{' => balance += 1,
            ')' | ']' | '}' if balance > 0 => balance -= 1,
            _ => {}
        }
    }
    balance
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
        FsCommands::Upload { src, dest, session } | FsCommands::Push { src, dest, session } => {
            handle_upload(config, ui, session, &src, Some(&dest)).await
        }
        FsCommands::Download { src, dest, session } | FsCommands::Pull { src, dest, session } => {
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
        FsCommands::Diff(args) => handle_fs_diff(args, ui, json),
        FsCommands::Changed(args) => handle_fs_diff(args, ui, json),
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
            preflight_timeout,
            retries,
            no_retry,
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
            drive_mount(
                config,
                ui,
                json,
                DriveMountOptions {
                    session,
                    path,
                    timeout_secs: timeout,
                    preflight_timeout_secs: preflight_timeout,
                    retries: if no_retry { 0 } else { retries },
                    open,
                },
            )
            .await
        }
        FsDriveCommands::Status { session, dry_run } => {
            if dry_run {
                return print_value(
                    json,
                    &serde_json::json!({
                        "action": "drive.status",
                        "needs_session": true,
                        "next_action": "run `colab fs drive mount --session NAME` if not mounted"
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
            preflight_timeout,
            retries,
            no_retry,
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
            drive_mount(
                config,
                ui,
                json,
                DriveMountOptions {
                    session,
                    path,
                    timeout_secs: timeout,
                    preflight_timeout_secs: preflight_timeout,
                    retries: if no_retry { 0 } else { retries },
                    open,
                },
            )
            .await
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

struct DriveMountOptions {
    session: Option<String>,
    path: String,
    timeout_secs: u64,
    preflight_timeout_secs: u64,
    retries: u8,
    open: bool,
}

async fn drive_mount(
    config: &ColabConfig,
    ui: Ui,
    json: bool,
    options: DriveMountOptions,
) -> Result<()> {
    let DriveMountOptions {
        session,
        path,
        timeout_secs,
        preflight_timeout_secs,
        retries,
        open,
    } = options;
    drive_progress(&ui, "Drive mount");
    drive_stage(&ui, "load_selected_session", "checking session");
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    debug::debug1(format!(
        "session store loaded sessions={} selected={:?}",
        servers.len(),
        session.as_deref().unwrap_or("<auto>")
    ));
    let server = resolve_server(&servers, session.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    debug::debug1(format!(
        "drive.mount stage=load_session ok name={:?}",
        server.label
    ));
    drive_done(&ui, "session loaded", &server.label);

    drive_stage(&ui, "validate_endpoint_url", "validating endpoint");
    validate_runtime_endpoint(&server)?;
    debug::debug1(format!(
        "drive.mount stage=validate_endpoint ok endpoint={}",
        server.endpoint
    ));
    debug::debug2(format!("session endpoint host={}", server.endpoint));
    drive_done(&ui, "endpoint url valid", &server.endpoint);

    let preflight_timeout = std::time::Duration::from_secs(preflight_timeout_secs.max(1));
    let sessions = drive_jupyter_sessions_with_retry(
        manager.client(),
        &server,
        preflight_timeout,
        retries,
        &ui,
        json,
    )
    .await
    .inspect_err(log_drive_failure)?;
    drive_done(&ui, "endpoint reachable", "Jupyter sessions API");

    drive_stage(&ui, "find_kernel", "finding kernel");
    let session = select_drive_kernel(&sessions)?;
    drive_done(&ui, "kernel found", "python3");

    drive_stage(&ui, "check_existing_mount", "checking existing Drive mount");
    let status = drive_status_for_server(manager.client(), &server, &path).await?;
    if status.mounted == Some(true) {
        return print_drive_mount_success(json, &path, "Drive already mounted");
    }

    if open {
        let url = crate::cocli::runtime::session_url(&config.colab_domain, &server.endpoint)
            .map_err(|e| ColabError::config(e.to_string()))?;
        open_url(&url)?;
        ui.info("opened browser");
    }

    let timeout = std::time::Duration::from_secs(timeout_secs.max(1));
    drive_stage(&ui, "verify_kernel_context", "checking kernel context");
    preflight_drive_kernel(manager.client(), &server, session, timeout).await?;

    drive_stage(&ui, "request_drive_mount", "requesting Drive mount");
    drive_stage(
        &ui,
        "wait_for_browser_approval",
        "waiting for browser approval if needed",
    );
    let output = runner::execute_colab_cell_in_session(
        manager.client(),
        &server,
        session,
        &drive_mount_cell(&path),
        timeout,
    )
    .await
    .map_err(|e| map_drive_stage_error("request_drive_mount", &server, e))?;
    drive_output_to_result(&output)?;

    if output.timed_out {
        return Err(drive_approval_required(Some(output.raw_text())));
    }

    drive_stage(&ui, "verify_drive_path", "verifying /content/drive");
    let after = drive_status_for_server(manager.client(), &server, &path).await?;
    if after.mounted == Some(true) || drive_mount_output_looks_ok(&output.raw_text()) {
        print_drive_mount_success(json, &path, "Drive mounted")
    } else {
        Err(ColabError::drive(
            "drive_status_unknown",
            "Could not confirm Drive status after mount",
            Some("colab fs drive status"),
            Some(output.raw_text()),
        ))
    }
}

fn drive_progress(ui: &Ui, title: &str) {
    if !ui.quiet && !debug::enabled(1) {
        println!("{title}");
        println!("{}", rule(*ui));
        println!();
    }
}

fn drive_stage(ui: &Ui, _stage: &str, label: &str) {
    if !ui.quiet && !debug::enabled(1) {
        println!("· {label}");
    }
}

fn drive_done(ui: &Ui, label: &str, detail: &str) {
    if !ui.quiet && !debug::enabled(1) {
        println!("✓ {label:<26} {detail}");
    }
}

fn validate_runtime_endpoint(server: &StoredServer) -> Result<()> {
    if server.endpoint.trim().is_empty() || server.proxy_url.trim().is_empty() {
        return Err(ColabError::drive_stage(
            "invalid_runtime_endpoint",
            "Runtime endpoint is missing from the local session record",
            "validate_endpoint_url",
            false,
            vec![
                "colab session list --refresh".to_string(),
                "colab session new --name work".to_string(),
            ],
            None,
        ));
    }
    if !server.proxy_url.starts_with("https://") && !server.proxy_url.starts_with("http://") {
        return Err(ColabError::drive_stage(
            "invalid_runtime_endpoint",
            "Runtime endpoint URL is invalid",
            "validate_endpoint_url",
            false,
            vec![
                "colab session repair".to_string(),
                "colab session new --name work".to_string(),
            ],
            Some(server.proxy_url.clone()),
        ));
    }
    Ok(())
}

async fn drive_jupyter_sessions_with_retry(
    client: &ColabClient,
    server: &StoredServer,
    timeout: std::time::Duration,
    retries: u8,
    ui: &Ui,
    json: bool,
) -> Result<Vec<Session>> {
    let attempts = retries.saturating_add(1).max(1);
    let mut last = None;
    for attempt in 1..=attempts {
        debug::debug1(format!(
            "drive.mount stage=check_jupyter_sessions attempt={attempt}/{attempts}"
        ));
        debug::debug2(format!(
            "http request method=GET path=/api/sessions timeout={}s",
            timeout.as_secs_f64()
        ));
        debug::debug3(format!(
            "http request url={}",
            debug::sanitize_url(&format!(
                "https://colab.research.google.com/tun/m/{}/api/sessions?authuser=0",
                server.endpoint
            ))
        ));
        drive_stage(
            ui,
            "check_jupyter_sessions",
            &format!("checking Jupyter sessions attempt {attempt}/{attempts}"),
        );
        let started = std::time::Instant::now();
        match tokio::time::timeout(timeout, list_runtime_sessions(client, server)).await {
            Ok(Ok(sessions)) => {
                debug::debug1(format!(
                    "drive.mount stage=check_jupyter_sessions ok elapsed={:.3}s sessions={}",
                    started.elapsed().as_secs_f64(),
                    sessions.len()
                ));
                return Ok(sessions);
            }
            Ok(Err(e)) => {
                let mapped = map_drive_stage_error("check_jupyter_sessions", server, e);
                debug::debug1(format!(
                    "http error elapsed={:.3}s retryable={}",
                    started.elapsed().as_secs_f64(),
                    yes_no(drive_error_retryable(&mapped))
                ));
                if !drive_error_retryable(&mapped) || attempt == attempts {
                    return Err(mapped);
                }
                last = Some(mapped);
            }
            Err(_) => {
                let mapped =
                    drive_endpoint_error("check_jupyter_sessions", server, "timeout", true, None);
                debug::debug1(format!(
                    "http timeout method=GET path=/api/sessions elapsed={:.3}s retryable=yes",
                    started.elapsed().as_secs_f64()
                ));
                if attempt == attempts {
                    return Err(mapped);
                }
                last = Some(mapped);
            }
        }
        let jitter_ms = (u64::from(attempt) * 37) % 100;
        let backoff = std::time::Duration::from_millis(200 * u64::from(attempt) + jitter_ms);
        debug::debug1(format!(
            "retry scheduled attempt={}/{} backoff={}ms",
            attempt + 1,
            attempts,
            backoff.as_millis()
        ));
        debug::debug2(format!(
            "retry backoff={}ms jitter={}ms",
            backoff.as_millis(),
            jitter_ms
        ));
        if !json && !ui.quiet && !debug::enabled(1) {
            println!("· retrying runtime endpoint");
        }
        tokio::time::sleep(backoff).await;
    }
    Err(last.unwrap_or_else(|| {
        drive_endpoint_error(
            "check_jupyter_sessions",
            server,
            "unknown_network",
            true,
            None,
        )
    }))
}

fn select_drive_kernel(sessions: &[Session]) -> Result<&Session> {
    sessions.iter().find(|s| s.kernel.is_some()).ok_or_else(|| {
        ColabError::drive_stage(
            "drive_kernel_context_required",
            "Drive mount needs a Colab kernel session",
            "find_kernel",
            false,
            vec!["colab session url --open".to_string()],
            None,
        )
    })
}

fn drive_error_retryable(error: &ColabError) -> bool {
    matches!(error, ColabError::Drive(drive) if drive.retryable)
}

fn log_drive_failure(error: &ColabError) {
    if let ColabError::Drive(drive) = error {
        debug::debug1(format!(
            "drive.mount failed kind={} stage={} retryable={}",
            drive.kind,
            drive.stage.as_deref().unwrap_or("<unknown>"),
            yes_no(drive.retryable)
        ));
        if let Some(raw) = &drive.raw {
            debug::debug3(format!("drive.mount raw={}", trim_raw(&debug::redact(raw))));
        }
    }
}

fn map_drive_stage_error(stage: &str, server: &StoredServer, error: ColabError) -> ColabError {
    match error {
        ColabError::ApiError { status, url, body } => {
            let kind = match status {
                401 | 403 => "runtime_endpoint_auth",
                404 => "stale_runtime_endpoint",
                500..=599 => "colab_busy",
                _ => "runtime_endpoint_unreachable",
            };
            let retryable = matches!(status, 429 | 500 | 502 | 503 | 504);
            drive_endpoint_error(
                stage,
                server,
                kind,
                retryable,
                Some(format!("{url}\n{body:?}")),
            )
        }
        ColabError::Network(e) => {
            let class = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "runtime_endpoint_unreachable"
            } else {
                classify_network_error(&e.to_string())
            };
            let raw = e
                .url()
                .map(|url| format!("{} url={}", e, debug::sanitize_url(url.as_str())))
                .unwrap_or_else(|| e.to_string());
            drive_endpoint_error(stage, server, class, class != "tls", Some(raw))
        }
        ColabError::ServerNotFound { endpoint } => drive_endpoint_error(
            stage,
            server,
            "stale_runtime_endpoint",
            false,
            Some(endpoint),
        ),
        other => other,
    }
}

fn map_session_network_error(stage: &str, error: ColabError) -> ColabError {
    match error {
        ColabError::ApiError { status, url, body } => {
            let retryable = retryable_status(status);
            ColabError::drive_stage(
                "session_refresh_failed",
                format!("Colab returned {status} {}", http_reason(status)),
                stage,
                retryable,
                vec![
                    "colab session list --refresh".to_string(),
                    "colab session new --name work".to_string(),
                ],
                Some(format!("{url}\n{body:?}")),
            )
        }
        ColabError::Network(e) => ColabError::drive_stage(
            classify_network_error(&e.to_string()),
            "Session refresh could not reach Colab",
            stage,
            true,
            vec!["colab session list --refresh".to_string()],
            Some(e.to_string()),
        ),
        other => other,
    }
}

fn drive_endpoint_error(
    stage: &str,
    server: &StoredServer,
    kind: &str,
    retryable: bool,
    raw: Option<String>,
) -> ColabError {
    let kind = if kind == "timeout" {
        "runtime_endpoint_timeout"
    } else {
        kind
    };
    let message = match kind {
        "runtime_endpoint_timeout" => "Runtime endpoint timed out",
        "dns" => "Runtime endpoint DNS lookup failed",
        "connection_refused" => "Runtime endpoint refused the connection",
        "tls" => "Runtime endpoint TLS handshake failed",
        "runtime_endpoint_auth" => "Runtime endpoint rejected the current credentials",
        "stale_runtime_endpoint" => "Runtime endpoint is stale",
        "colab_busy" => "Colab is busy",
        _ => "Runtime endpoint is not reachable",
    };
    ColabError::drive_stage(
        kind.to_string(),
        format!(
            "{message}\nsession: {}\n\nThis usually means the runtime expired, the endpoint changed, or Colab is busy",
            server.label
        ),
        stage.to_string(),
        retryable,
        vec![
            "colab session list --refresh".to_string(),
            "colab session new --name work".to_string(),
        ],
        raw,
    )
}

fn classify_network_error(raw: &str) -> &'static str {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("dns") || lower.contains("failed to lookup") {
        "dns"
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "timeout"
    } else if lower.contains("connection refused") {
        "connection_refused"
    } else if lower.contains("tls") || lower.contains("certificate") {
        "tls"
    } else {
        "unknown_network"
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
    validate_runtime_endpoint(&server)?;
    let sessions = drive_jupyter_sessions_with_retry(
        manager.client(),
        &server,
        std::time::Duration::from_secs(10),
        0,
        &ui,
        json,
    )
    .await?;
    let session = select_drive_kernel(&sessions)?;
    preflight_drive_kernel(
        manager.client(),
        &server,
        session,
        std::time::Duration::from_secs(30),
    )
    .await?;

    let output = runner::execute_colab_cell_in_session(
        manager.client(),
        &server,
        session,
        "from google.colab import drive\ndrive.flush_and_unmount()",
        std::time::Duration::from_secs(60),
    )
    .await?;
    drive_output_to_result(&output)?;
    if output.timed_out {
        return Err(ColabError::drive(
            "drive_unmount_timeout",
            "Drive unmount did not finish before timeout",
            Some("colab fs drive status"),
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
    drive_status_for_server(manager.client(), &server, path).await
}

async fn drive_status_for_server(
    client: &ColabClient,
    server: &StoredServer,
    path: &str,
) -> Result<DriveStatus> {
    let out =
        match runner::capture_remote_command(client, server, &drive_status_probe_command(path))
            .await
        {
            Ok(out) => out,
            Err(_) => {
                return Ok(DriveStatus {
                    ok: false,
                    mounted: None,
                    path: path.to_string(),
                    next_action: Some("colab status check".to_string()),
                });
            }
        };
    Ok(parse_drive_status(&out, path))
}

async fn preflight_drive_kernel(
    client: &ColabClient,
    server: &StoredServer,
    session: &Session,
    timeout: std::time::Duration,
) -> Result<()> {
    let output = runner::execute_colab_cell_in_session(
        client,
        server,
        session,
        drive_preflight_cell(),
        timeout,
    )
    .await
    .map_err(|e| map_drive_stage_error("verify_kernel_context", server, e))?;
    drive_output_to_result(&output)?;
    if output.timed_out {
        return Err(ColabError::drive(
            "drive_kernel_timeout",
            "Drive mount needs a responsive Colab kernel session",
            Some("colab session url --open"),
            Some(output.raw_text()),
        ));
    }
    if output.stdout.trim() == "true" {
        Ok(())
    } else {
        Err(ColabError::drive(
            "drive_kernel_context_required",
            "Drive mount needs a Colab kernel session, not a plain Python process",
            Some("colab session url --open"),
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
                println!("fix: {next}");
            }
        }
        None => {
            println!("Could not confirm Drive status");
            if let Some(next) = &status.next_action {
                println!("fix: {next}");
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
            next_action: Some("colab fs drive mount".to_string()),
        },
        _ => DriveStatus {
            ok: false,
            mounted: None,
            path: path.to_string(),
            next_action: Some("colab status check".to_string()),
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
            Some("colab fs drive status"),
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
            Some("colab session url --open"),
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
            Some("colab status check"),
            Some(raw.to_string()),
        ));
    }
    None
}

fn drive_approval_required(raw: Option<String>) -> ColabError {
    ColabError::drive(
        "drive_browser_approval_required",
        "Drive needs browser approval",
        Some("open the session once, then run fs drive mount again: colab session url --open"),
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

async fn handle_env(
    cmd: EnvCommands,
    config: &ColabConfig,
    ui: Ui,
    secrets: &SecretBundle,
) -> Result<()> {
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
                secrets,
            )
            .await
        }
        EnvCommands::Freeze { session } => {
            handle_run(
                config,
                ui,
                session,
                vec!["python".into(), "-m".into(), "pip".into(), "freeze".into()],
                secrets,
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
                secrets,
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
            ui.info("GPU details require a session; use `colab run py --code \"import torch; print(torch.cuda.get_device_name(0))\"`.");
            Ok(())
        }
        RuntimeCommands::Tpu => {
            ui.info(
                "TPU details require a session; use `colab status runtime --backend` for package baselines.",
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
    print_value_or_kv(json, "runtime backend", &data)
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
            let report = build_status_report(config)?;
            render_status_report(&report, json, ui)
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
            print_value_or_kv(
                json,
                "auth",
                &serde_json::json!({
                    "signed_in": auth_state.is_some(),
                    "email": auth_state.as_ref().map(|email| crate::cocli::auth::redaction::redacted_email(email, false)),
                    "next_action": if auth_state.is_some() { "run `colab session list`" } else { "run `colab auth login`" }
                }),
            )
        }
        Some(StatusCommands::Fs) => print_value_or_kv(
            json,
            "files",
            &serde_json::json!({
                "sync": "manifest dry-run available",
                "next_action": "run `colab fs changed LOCAL REMOTE`"
            }),
        ),
        Some(StatusCommands::Drive) => print_value_or_kv(
            json,
            "drive",
            &serde_json::json!({
                "status": "needs live session",
                "next_action": "run `colab fs drive status --session NAME`"
            }),
        ),
        Some(StatusCommands::Kernel {
            all,
            refresh,
            session,
        }) => {
            if refresh || all {
                return handle_session_kernel(
                    config,
                    ui,
                    json,
                    if all {
                        SessionKernelCommands::List(KernelSessionArg { session })
                    } else {
                        SessionKernelCommands::Current(KernelSessionArg { session })
                    },
                )
                .await;
            }
            let manager = make_manager(config)?;
            let servers = manager.list_local()?;
            let server = resolve_server(&servers, session.as_deref())?;
            let language = server
                .kernel_language
                .as_deref()
                .map(KernelLanguage::detect)
                .unwrap_or_else(|| KernelLanguage::Unknown("unknown".to_string()));
            let view = KernelView {
                selected: true,
                name: server
                    .selected_kernel_name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                language,
                version: server.kernel_language_version.clone(),
                state: if server.kernel_cache_stale {
                    "stale".to_string()
                } else {
                    "cached".to_string()
                },
                id: server
                    .selected_kernel_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                session_id: server.label.clone(),
            };
            print_current_kernel(&view, json, ui)
        }
        Some(StatusCommands::Slurp { config }) => print_value_or_kv(
            json,
            "recipe",
            &serde_json::json!({
                "config": config,
                "exists": Path::new(&config).exists(),
                "next_action": if Path::new(&config).exists() { "run `colab distribute recipe explain`" } else { "run `colab distribute recipe init`" }
            }),
        ),
        Some(StatusCommands::Fleet { config }) => {
            let cfg = load_cocli_config().unwrap_or_default();
            if !cfg.experiments.distribute {
                return print_value_or_kv(
                    json,
                    "distribute",
                    &serde_json::json!({
                        "enabled": false,
                        "experimental": true,
                        "fix": "colab settings experiments"
                    }),
                );
            }
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
                print_value_or_kv(
                    json,
                    "distribute",
                    &serde_json::json!({
                        "config": config,
                        "exists": false,
                        "next_action": "run `colab distribute recipe init`"
                    }),
                )
            }
        }
        Some(StatusCommands::Quick) => {
            let report = build_status_report(config)?;
            render_status_report(&report, json, ui)
        }
        Some(StatusCommands::Check) => {
            let mut report = build_status_report(config)?;
            report.title = "colab check".to_string();
            render_status_report(&report, json, ui)
        }
        Some(StatusCommands::Run) => print_value_or_kv(
            json,
            "run",
            &serde_json::json!({
                "note": "runtime setup checks require a live session",
                "next_action": "run `colab run py --session NAME --code \"import sys; print(sys.version)\"`"
            }),
        ),
        Some(StatusCommands::Paths) => print_value_or_kv(
            json,
            "paths",
            &serde_json::json!({
                "config_dir": config::config_dir().ok().map(|p| p.display().to_string()),
                "data_dir": config::data_dir().ok().map(|p| p.display().to_string()),
                "config_path": config::config_path().ok().map(|p| p.display().to_string())
            }),
        ),
        Some(StatusCommands::Version) => print_version_info(json),
    }
}

fn print_version_info(json: bool) -> Result<()> {
    let config_path = config::config_path().ok().map(|p| p.display().to_string());
    let data = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "git_sha": option_env!("VERGEN_GIT_SHA").or(option_env!("GIT_SHA")),
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "features": build_feature_list(),
        "config_path": config_path,
    });
    print_value_or_kv(json, "version", &data)
}

fn build_feature_list() -> Vec<&'static str> {
    [
        (cfg!(feature = "dev-tools"), "dev-tools"),
        (cfg!(feature = "owner-tools"), "owner-tools"),
    ]
    .into_iter()
    .filter_map(|(enabled, name)| enabled.then_some(name))
    .collect()
}

#[derive(Debug, serde::Serialize)]
struct StatusReport {
    title: String,
    sections: Vec<StatusLine>,
    fix: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct StatusLine {
    name: &'static str,
    state: &'static str,
    message: String,
}

fn build_status_report(config: &ColabConfig) -> Result<StatusReport> {
    let account = auth::current_account()?;
    let sessions = local_servers(config);
    let has_session = !sessions.is_empty();
    let files_ready = cache_writable(&config.data_dir);

    let cfg = load_cocli_config().unwrap_or_default();
    let recipe_exists = Path::new("colab.recipe.toml").exists() || Path::new("slurp.toml").exists();
    let fix = if account.is_none() {
        Some("run colab auth login".to_string())
    } else if !has_session {
        Some("run colab session list".to_string())
    } else if !files_ready {
        Some("check local data directory permissions".to_string())
    } else {
        None
    };

    Ok(StatusReport {
        title: "colab status".to_string(),
        sections: vec![
            StatusLine {
                name: "Auth",
                state: if account.is_some() { "ready" } else { "warn" },
                message: if account.is_some() {
                    "ready".to_string()
                } else {
                    "sign in to continue".to_string()
                },
            },
            StatusLine {
                name: "Session",
                state: if has_session { "ready" } else { "warn" },
                message: sessions
                    .iter()
                    .max_by_key(|s| s.date_assigned)
                    .map(|s| format!("selected: {}", s.label))
                    .unwrap_or_else(|| "none selected".to_string()),
            },
            StatusLine {
                name: "Runtime",
                state: if has_session { "info" } else { "idle" },
                message: if has_session {
                    "check with status runtime --all".to_string()
                } else {
                    "pick a session first".to_string()
                },
            },
            StatusLine {
                name: "Files",
                state: if files_ready { "ready" } else { "warn" },
                message: if files_ready {
                    "cache writable".to_string()
                } else {
                    "cache path is not writable".to_string()
                },
            },
            StatusLine {
                name: "Drive",
                state: "idle",
                message: "not checked".to_string(),
            },
            StatusLine {
                name: "Recipe",
                state: if recipe_exists { "ready" } else { "idle" },
                message: if recipe_exists {
                    "recipe found".to_string()
                } else {
                    "no recipe".to_string()
                },
            },
            StatusLine {
                name: "Distribute",
                state: "idle",
                message: if cfg.experiments.distribute {
                    "on".to_string()
                } else {
                    "off".to_string()
                },
            },
            StatusLine {
                name: "Continue",
                state: "idle",
                message: if cfg.experiments.continue_work {
                    "on".to_string()
                } else {
                    "off".to_string()
                },
            },
        ],
        fix,
    })
}

fn cache_writable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

fn local_servers(config: &ColabConfig) -> Vec<StoredServer> {
    std::fs::read_to_string(config.servers_file())
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default()
}

fn render_status_report(report: &StatusReport, json: bool, ui: Ui) -> Result<()> {
    if json {
        return print_value(true, report);
    }
    if ui.quiet {
        for section in &report.sections {
            println!(
                "{}\t{}\t{}",
                section.name.to_ascii_lowercase(),
                section.state,
                section.message
            );
        }
        if let Some(fix) = &report.fix {
            println!("fix\t{fix}");
        }
        return Ok(());
    }
    println!("{}", heading(&report.title, ui));
    println!("{}", rule(ui));
    println!();
    for section in &report.sections {
        let symbol = status_symbol(section.state, ui);
        println!(
            "{:<10} {} {}",
            section.name,
            symbol,
            human_message(&section.message, ui)
        );
    }
    if let Some(fix) = &report.fix {
        println!();
        println!("fix: {}", command_text(fix, ui));
    }
    Ok(())
}

fn status_symbol(state: &str, ui: Ui) -> String {
    if ui.plain {
        return match state {
            "ready" => "✓",
            "warn" | "missing" => "!",
            "error" => "x",
            _ => "·",
        }
        .to_string();
    }
    match state {
        "ready" => "✓".bright_green().bold().to_string(),
        "warn" | "missing" => "!".yellow().bold().to_string(),
        "error" => "x".bright_red().bold().to_string(),
        "info" => "·".bright_cyan().to_string(),
        _ => "·".dimmed().to_string(),
    }
}

fn heading(text: &str, ui: Ui) -> String {
    if ui.plain {
        text.to_string()
    } else {
        text.bright_magenta().bold().to_string()
    }
}

fn rule(ui: Ui) -> String {
    let line = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
    if ui.plain {
        line.to_string()
    } else {
        line.bright_blue().to_string()
    }
}

fn command_text(text: &str, ui: Ui) -> String {
    if ui.plain {
        text.to_string()
    } else {
        text.bright_cyan().to_string()
    }
}

fn path_text(text: &str, ui: Ui) -> String {
    if ui.plain {
        text.to_string()
    } else {
        text.bright_blue().to_string()
    }
}

fn human_message(text: &str, ui: Ui) -> String {
    if let Some(path) = text.strip_prefix('/') {
        return path_text(&format!("/{path}"), ui);
    }
    text.to_string()
}

fn print_value_or_kv<T: serde::Serialize>(json: bool, title: &str, value: &T) -> Result<()> {
    if json {
        return print_value(true, value);
    }
    let value = serde_json::to_value(value)?;
    println!("{title}");
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                println!("  {:<14} {}", human_key(&key), human_value(&value));
            }
        }
        other => println!("  {}", human_value(&other)),
    }
    Ok(())
}

fn human_key(key: &str) -> String {
    key.replace('_', " ")
}

fn human_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "<invalid>".to_string()),
    }
}

fn handle_tools(cmd: ToolsCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        ToolsCommands::List { json: local_json } => handle_skills(
            SkillCommands::List {
                json: local_json,
                category: None,
                scope: None,
                risk: None,
                needs_session: false,
                enabled: false,
                disabled: false,
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

fn handle_settings(cmd: Option<SettingsCommands>, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        None => print_settings_overview(json, ui),
        Some(SettingsCommands::Get { key: None }) => {
            if json {
                handle_config(ConfigCommands::Get, true)
            } else {
                print_settings_overview(false, ui)
            }
        }
        Some(SettingsCommands::Get { key: Some(key) }) => handle_config_get_key(&key, json),
        Some(SettingsCommands::Set { key, value }) => {
            handle_config(ConfigCommands::Set { key, value }, json)
        }
        Some(SettingsCommands::Path) => handle_config(ConfigCommands::Path, json),
        Some(SettingsCommands::Edit) => handle_config(ConfigCommands::Open, json),
        Some(SettingsCommands::Reset { yes }) => {
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
        Some(SettingsCommands::Skills { command }) => handle_skills(command, ui, json),
        Some(SettingsCommands::Ui { command }) => handle_settings_ui(command, ui, json),
        Some(SettingsCommands::Experiments { command }) => {
            handle_settings_experiments(command, ui, json)
        }
        Some(SettingsCommands::Support { command }) => handle_settings_support(command, json),
        Some(SettingsCommands::About) => print_version_info(json),
        Some(SettingsCommands::Update { command }) => handle_settings_update(command, json),
        Some(SettingsCommands::Billing { command }) => handle_settings_billing(command, json),
        #[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
        Some(SettingsCommands::Dev { command }) => {
            if !dev_tools_unlocked()? {
                return Err(ColabError::config("private maintainer command"));
            }
            match command {
                DevCommands::Release { command } => handle_release(command, json),
            }
        }
    }
}

fn handle_settings_update(command: SettingsUpdateCommands, json: bool) -> Result<()> {
    match command {
        SettingsUpdateCommands::Check => {
            let manager = detect_install_manager();
            print_value_or_kv(
                json,
                "update",
                &serde_json::json!({
                    "current": env!("CARGO_PKG_VERSION"),
                    "installer": manager,
                    "auto_install": false,
                    "note": "update check is local unless an installer-specific updater is configured"
                }),
            )
        }
        SettingsUpdateCommands::Install { yes } => {
            if !yes {
                return Err(ColabError::config(
                    "update install requires --yes; colab will not self-modify blindly",
                ));
            }
            Err(ColabError::config(
                "automatic update install is not configured for this binary\nfix: reinstall with your package manager",
            ))
        }
    }
}

fn handle_update(install: bool, yes: bool, json: bool) -> Result<()> {
    if install {
        handle_settings_update(SettingsUpdateCommands::Install { yes }, json)
    } else {
        handle_settings_update(SettingsUpdateCommands::Check, json)
    }
}

fn detect_install_manager() -> &'static str {
    let exe = std::env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    if exe.contains(".cargo/bin") {
        "cargo"
    } else if exe.contains("homebrew") || exe.contains("/Cellar/") {
        "homebrew"
    } else if exe.contains(".local/bin") || exe.contains("uv") {
        "uv-or-pip"
    } else {
        "unknown"
    }
}

fn handle_settings_billing(command: SettingsBillingCommands, json: bool) -> Result<()> {
    match command {
        SettingsBillingCommands::Open { dry_run } => {
            let url = "https://colab.research.google.com/signup";
            if json {
                return print_value(
                    true,
                    &serde_json::json!({
                        "action": "open_billing",
                        "url": url,
                        "would_open": !dry_run,
                    }),
                );
            }
            if dry_run {
                println!("billing");
                println!("  open           {url}");
                println!("  would open     no");
                return Ok(());
            }
            open_url(url)?;
            println!("opened Colab billing page");
            Ok(())
        }
        SettingsBillingCommands::Status => print_value_or_kv(
            json,
            "billing",
            &serde_json::json!({
                "available": false,
                "message": "billing status unavailable from local config",
                "open": "colab pay"
            }),
        ),
    }
}

fn print_settings_overview(json: bool, ui: Ui) -> Result<()> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    let cfg = config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
    if json {
        return print_value(true, &cfg);
    }
    let rows = [
        ("General", "config path, profiles, defaults"),
        ("UI", "colour, theme, animations, bell, fun"),
        ("Experiments", "disabled optional features"),
        ("AI", "agent and tool workflows"),
        ("Auth", "ADC, OAuth2, and profiles"),
        ("Support", "redacted bug reports and bundles"),
        ("Dev", "maintainer-only tools, hidden by default"),
    ];

    if ui.interactive {
        return run_settings_editor(path, cfg, SettingsPage::Main, ui);
    }
    print_settings_menu(&path, &cfg, ui, &rows)
}

fn print_settings_menu(
    path: &Path,
    cfg: &config::CocliConfig,
    ui: Ui,
    rows: &[(&str, &str)],
) -> Result<()> {
    println!("{}", heading("Settings", ui));
    println!("Config, UI, support, and experiments");
    println!();
    for (index, (name, note)) in rows
        .iter()
        .filter(|(name, _)| *name != "Dev" || dev_visible(cfg))
        .enumerate()
    {
        let marker = if index == 0 { "›" } else { " " };
        println!("{marker} {:<14} {}", name, muted(note, ui));
    }
    println!();
    println!("Config path");
    println!("  {}", path_text(&path.display().to_string(), ui));
    println!();
    println!("Current");
    println!("  color           {}", color_choice_name(cfg.ui.color));
    println!("  theme           {}", cfg.ui.theme);
    println!("  animations      {}", on_off(cfg.ui.animations));
    println!("  tui             {}", cfg.ui.tui);
    println!("  bell            {}", on_off(cfg.ui.bell));
    println!("  experiments     {}", experiments_summary(cfg));
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsPage {
    Main,
    General,
    Ui,
    Experiments,
    Ai,
    Auth,
    Support,
    Dev,
}

const SETTINGS_FOOTER: &str = "↑/↓ move · enter open/toggle · ←/→ change · space toggle · b/esc back · s save · q quit · ? help";

fn run_settings_editor(
    path: PathBuf,
    cfg: config::CocliConfig,
    start: SettingsPage,
    ui: Ui,
) -> Result<()> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEvent},
        execute,
        terminal::{self, ClearType},
    };

    terminal::enable_raw_mode().map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
    let _raw = LocalRawModeGuard;
    let mut state = SettingsEditorState::new(path, cfg, start);
    let mut stdout = std::io::stdout();

    loop {
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )
        .map_err(|e| ColabError::config(format!("terminal render: {e}")))?;
        let text = settings_editor_text(&state, ui, crate::cocli::ui::width::terminal_width());
        stdout.write_all(text.replace('\n', "\r\n").as_bytes())?;
        stdout.flush()?;

        let Event::Key(KeyEvent { code, .. }) =
            event::read().map_err(|e| ColabError::config(format!("terminal read: {e}")))?
        else {
            continue;
        };
        match code {
            KeyCode::Up => state.move_by(-1),
            KeyCode::Down => state.move_by(1),
            KeyCode::Left => state.change_selected(-1),
            KeyCode::Right => state.change_selected(1),
            KeyCode::Enter | KeyCode::Char(' ') => state.activate_selected(),
            KeyCode::Char('s') => state.save()?,
            KeyCode::Char('?') => state.message = Some(SETTINGS_FOOTER.to_string()),
            KeyCode::Char('b') | KeyCode::Esc => {
                if state.back_or_exit()? {
                    break;
                }
            }
            KeyCode::Char('q') => {
                if !state.dirty || confirm_discard()? {
                    break;
                }
            }
            _ => {}
        }
    }
    execute!(stdout, cursor::MoveToColumn(0))
        .map_err(|e| ColabError::config(format!("terminal render: {e}")))?;
    Ok(())
}

struct SettingsEditorState {
    path: PathBuf,
    cfg: config::CocliConfig,
    original: config::CocliConfig,
    page: SettingsPage,
    stack: Vec<SettingsPage>,
    selected: usize,
    dirty: bool,
    message: Option<String>,
}

impl SettingsEditorState {
    fn new(path: PathBuf, cfg: config::CocliConfig, page: SettingsPage) -> Self {
        Self {
            path,
            original: cfg.clone(),
            cfg,
            page,
            stack: Vec::new(),
            selected: 0,
            dirty: false,
            message: None,
        }
    }

    fn len(&self) -> usize {
        match self.page {
            SettingsPage::Main => self.main_pages().len(),
            SettingsPage::Ui => 10,
            SettingsPage::Experiments => 8,
            _ => 1,
        }
    }

    fn main_pages(&self) -> Vec<(SettingsPage, &'static str, &'static str)> {
        let mut pages = vec![
            (SettingsPage::General, "General", "config path and defaults"),
            (SettingsPage::Ui, "UI", "colour, theme, motion, terminal"),
            (
                SettingsPage::Experiments,
                "Experiments",
                "optional features, off by default",
            ),
            (SettingsPage::Ai, "AI", "agent and tool workflows"),
            (SettingsPage::Auth, "Auth", "ADC, OAuth2, and profiles"),
            (SettingsPage::Support, "Support", "redacted bug reports"),
        ];
        if dev_visible(&self.cfg) {
            pages.push((SettingsPage::Dev, "Dev", "maintainer-only tools"));
        }
        pages
    }

    fn move_by(&mut self, delta: isize) {
        let len = self.len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(len as isize) as usize;
        self.message = None;
    }

    fn activate_selected(&mut self) {
        match self.page {
            SettingsPage::Main => {
                let pages = self.main_pages();
                if let Some((page, _, _)) = pages.get(self.selected) {
                    self.stack.push(self.page);
                    self.page = *page;
                    self.selected = 0;
                }
            }
            SettingsPage::Ui => self.change_ui_selected(1),
            SettingsPage::Experiments => self.toggle_experiment_selected(),
            _ => self.message = Some("nothing to edit on this page".to_string()),
        }
    }

    fn change_selected(&mut self, delta: isize) {
        match self.page {
            SettingsPage::Ui => self.change_ui_selected(delta),
            _ => self.message = Some("left/right changes enum settings".to_string()),
        }
    }

    fn change_ui_selected(&mut self, delta: isize) {
        match self.selected {
            0 => {
                self.cfg.ui.color = cycle_color(self.cfg.ui.color, delta);
                self.mark_dirty();
            }
            1 => {
                self.cfg.ui.neon = !self.cfg.ui.neon;
                self.mark_dirty();
            }
            2 => {
                self.cfg.ui.theme = cycle_string(
                    &self.cfg.ui.theme,
                    &["auto", "light", "dark", "contrast"],
                    delta,
                );
                self.mark_dirty();
            }
            3 => toggle(&mut self.cfg.ui.animations),
            4 => toggle(&mut self.cfg.ui.bell),
            5 => toggle(&mut self.cfg.ui.fun),
            6 => toggle(&mut self.cfg.ui.compact),
            7 => toggle(&mut self.cfg.ui.icons),
            8 => toggle(&mut self.cfg.ui.unicode),
            9 => {
                self.cfg.ui.tui =
                    cycle_string(&self.cfg.ui.tui, &["auto", "always", "never"], delta);
                self.mark_dirty();
            }
            _ => {}
        }
        if matches!(self.selected, 3..=8) {
            self.mark_dirty();
        }
    }

    fn toggle_experiment_selected(&mut self) {
        match self.selected {
            0 => self.cfg.experiments.continue_work = !self.cfg.experiments.continue_work,
            1 => {
                self.cfg.experiments.distribute = !self.cfg.experiments.distribute;
                self.cfg.experiments.fleet = self.cfg.experiments.distribute;
                if !self.cfg.experiments.distribute {
                    self.cfg.experiments.multi_login = false;
                }
            }
            2 if self.cfg.experiments.distribute => {
                self.cfg.experiments.multi_login = !self.cfg.experiments.multi_login;
            }
            2 => {
                self.message = Some("multi-login is locked until Distribute is on".to_string());
                return;
            }
            3 => self.cfg.experiments.mcp_server = !self.cfg.experiments.mcp_server,
            4 => self.cfg.experiments.ai_plan_runner = !self.cfg.experiments.ai_plan_runner,
            5 => self.cfg.experiments.ast_observer = !self.cfg.experiments.ast_observer,
            6 => self.cfg.experiments.secrets_bridge = !self.cfg.experiments.secrets_bridge,
            7 => {
                self.cfg.experiments.background_live_checks =
                    !self.cfg.experiments.background_live_checks;
            }
            _ => {}
        }
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.dirty = self.cfg != self.original;
        self.message = None;
    }

    fn save(&mut self) -> Result<()> {
        self.cfg
            .save(&self.path)
            .map_err(|e| ColabError::config(e.to_string()))?;
        self.original = self.cfg.clone();
        self.dirty = false;
        self.message = Some("saved".to_string());
        Ok(())
    }

    fn back_or_exit(&mut self) -> Result<bool> {
        if let Some(page) = self.stack.pop() {
            self.page = page;
            self.selected = 0;
            self.message = None;
            return Ok(false);
        }
        Ok(!self.dirty || confirm_discard()?)
    }
}

fn toggle(value: &mut bool) {
    *value = !*value;
}

fn cycle_color(color: config::ColorChoice, delta: isize) -> config::ColorChoice {
    let values = [
        config::ColorChoice::Auto,
        config::ColorChoice::Always,
        config::ColorChoice::Never,
    ];
    values[cycle_index(
        values.iter().position(|v| *v == color).unwrap_or(0),
        values.len(),
        delta,
    )]
}

fn cycle_string(current: &str, values: &[&str], delta: isize) -> String {
    values[cycle_index(
        values.iter().position(|v| *v == current).unwrap_or(0),
        values.len(),
        delta,
    )]
    .to_string()
}

fn cycle_index(current: usize, len: usize, delta: isize) -> usize {
    (current as isize + delta).rem_euclid(len as isize) as usize
}

fn confirm_discard() -> Result<bool> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEvent},
        terminal,
    };
    print!("\r\nDiscard changes? y/N ");
    std::io::stdout().flush()?;
    loop {
        let Event::Key(KeyEvent { code, .. }) =
            event::read().map_err(|e| ColabError::config(format!("terminal read: {e}")))?
        else {
            continue;
        };
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter | KeyCode::Esc => {
                terminal::enable_raw_mode()
                    .map_err(|e| ColabError::config(format!("raw mode: {e}")))?;
                return Ok(false);
            }
            _ => {}
        }
    }
}

fn settings_editor_text(state: &SettingsEditorState, ui: Ui, width: usize) -> String {
    let width = width.clamp(40, 140);
    let mut out = String::new();
    out.push_str(&format!(
        "{}{}\n",
        heading(settings_page_title(state.page), ui),
        if state.dirty { "  unsaved changes" } else { "" }
    ));
    out.push_str(&format!(
        "{}\n\n",
        crate::cocli::ui::width::truncate_end(settings_page_subtitle(state.page), width)
    ));
    match state.page {
        SettingsPage::Main => {
            for (idx, (_, label, desc)) in state.main_pages().iter().enumerate() {
                push_settings_row(&mut out, idx == state.selected, label, desc, ui, width);
            }
        }
        SettingsPage::General => {
            out.push_str("Config path\n");
            out.push_str(&format!(
                "  {}\n\n",
                path_text(
                    &crate::cocli::ui::width::truncate_middle(
                        &state.path.display().to_string(),
                        width.saturating_sub(2)
                    ),
                    ui
                )
            ));
            out.push_str(&format!(
                "color        {}\n",
                color_choice_name(state.cfg.ui.color)
            ));
            out.push_str(&format!("theme        {}\n", state.cfg.ui.theme));
            out.push_str(&format!(
                "experiments  {}\n",
                experiments_summary(&state.cfg)
            ));
        }
        SettingsPage::Ui => {
            let rows = [
                (
                    "Color mode",
                    color_choice_name(state.cfg.ui.color).to_string(),
                    "auto / always / never",
                ),
                (
                    "Neon accents",
                    on_off(state.cfg.ui.neon).to_string(),
                    "brighter accents",
                ),
                (
                    "Theme",
                    state.cfg.ui.theme.clone(),
                    "auto / light / dark / contrast",
                ),
                (
                    "Animations",
                    on_off(state.cfg.ui.animations).to_string(),
                    "interactive progress motion",
                ),
                (
                    "Terminal bell",
                    on_off(state.cfg.ui.bell).to_string(),
                    "optional bell after long jobs",
                ),
                (
                    "Fun lines",
                    on_off(state.cfg.ui.fun).to_string(),
                    "rare harmless interactive lines",
                ),
                (
                    "Compact output",
                    on_off(state.cfg.ui.compact).to_string(),
                    "less spacing",
                ),
                (
                    "Icons",
                    on_off(state.cfg.ui.icons).to_string(),
                    "small symbols in output",
                ),
                (
                    "Unicode",
                    on_off(state.cfg.ui.unicode).to_string(),
                    "smooth borders and glyphs",
                ),
                (
                    "TUI panels",
                    state.cfg.ui.tui.clone(),
                    "auto / always / never",
                ),
            ];
            for (idx, (label, value, desc)) in rows.iter().enumerate() {
                push_settings_row(
                    &mut out,
                    idx == state.selected,
                    label,
                    &format!("{value:<8} {desc}"),
                    ui,
                    width,
                );
            }
        }
        SettingsPage::Experiments => {
            for (idx, item) in experiment_items(&state.cfg).iter().enumerate() {
                let locked = idx == 2 && !state.cfg.experiments.distribute;
                let value = if locked {
                    "locked".to_string()
                } else {
                    on_off(item.enabled).to_string()
                };
                push_settings_row(
                    &mut out,
                    idx == state.selected,
                    item.label,
                    &format!("{value:<7} {}", item.risk),
                    ui,
                    width,
                );
            }
        }
        SettingsPage::Ai => {
            out.push_str("AI tools list is read-only by default.\n");
            out.push_str("MCP and plan runner are controlled under Experiments.\n");
        }
        SettingsPage::Auth => {
            out.push_str("ADC and OAuth2 profiles live under `colab auth`.\n");
            out.push_str("Tokens are not stored in config.toml.\n");
        }
        SettingsPage::Support => {
            out.push_str("Bug reports are redacted by default.\n");
            out.push_str("Use `colab settings support bug-report` for a bundle.\n");
        }
        SettingsPage::Dev => out.push_str("Private maintainer tools.\n"),
    }
    if let Some(message) = &state.message {
        out.push('\n');
        out.push_str(&format!(
            "{}\n",
            muted(&crate::cocli::ui::width::truncate_end(message, width), ui)
        ));
    }
    out.push('\n');
    out.push_str(&muted(
        &crate::cocli::ui::width::truncate_end(SETTINGS_FOOTER, width),
        ui,
    ));
    out.push('\n');
    out
}

fn push_settings_row(
    out: &mut String,
    selected: bool,
    label: &str,
    desc: &str,
    ui: Ui,
    width: usize,
) {
    let marker = if selected { ">" } else { " " };
    let label_width = 16usize.min(width.saturating_sub(4));
    let used = 2 + label_width + 1;
    let desc_width = width.saturating_sub(used).max(8);
    let label = crate::cocli::ui::width::truncate_end(label, label_width);
    let desc = crate::cocli::ui::width::truncate_end(desc, desc_width);
    out.push_str(&format!(
        "{marker} {label:<label_width$} {}\n",
        muted(&desc, ui)
    ));
}

fn settings_page_title(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::Main => "Settings",
        SettingsPage::General => "General",
        SettingsPage::Ui => "UI settings",
        SettingsPage::Experiments => "Experiments",
        SettingsPage::Ai => "AI",
        SettingsPage::Auth => "Auth",
        SettingsPage::Support => "Support",
        SettingsPage::Dev => "Dev",
    }
}

fn settings_page_subtitle(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::Main => "Config, UI, experiments, support",
        SettingsPage::Ui => "Changes are saved to config.toml",
        SettingsPage::Experiments => "Optional features are off by default",
        _ => "b/esc back · q quit",
    }
}

fn handle_config_get_key(key: &str, json: bool) -> Result<()> {
    let cfg = load_cocli_config()?;
    let value = serde_json::to_value(&cfg)?;
    let selected = key
        .split('.')
        .try_fold(&value, |current, part| current.get(part))
        .ok_or_else(|| ColabError::config(format!("unknown settings key: {key}")))?;
    print_value(json, selected)
}

fn load_cocli_config() -> Result<config::CocliConfig> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    let cfg = config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
    debug::debug2(format!("settings config loaded path={}", path.display()));
    Ok(cfg)
}

fn handle_settings_ui(command: Option<SettingsUiCommands>, ui: Ui, json: bool) -> Result<()> {
    match command {
        None => render_ui_settings(ui, json),
        Some(SettingsUiCommands::Get { key: None }) => {
            let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
            let cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            if json {
                print_value(true, &cfg.ui)
            } else {
                render_ui_settings(Ui::new(ui.quiet, ui.plain, false), false)
            }
        }
        Some(SettingsUiCommands::Get { key: Some(key) }) => {
            handle_config_get_key(&format!("ui.{}", normalize_ui_key(&key)), json)
        }
        Some(SettingsUiCommands::Set { key, value }) => {
            let key = normalize_ui_key(&key);
            handle_config(
                ConfigCommands::Set {
                    key: format!("ui.{key}"),
                    value,
                },
                json,
            )
        }
        Some(SettingsUiCommands::Reset) => {
            let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
            let mut cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            cfg.ui = config::UiConfig::default();
            cfg.save(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            Ok(())
        }
        Some(SettingsUiCommands::Preview) => {
            if json {
                print_value(
                    true,
                    &serde_json::json!({
                        "theme": "neon",
                        "success": "electric mint",
                        "warning": "amber",
                        "error": "hot coral",
                        "info": "cyan",
                        "accent": "violet"
                    }),
                )
            } else {
                println!("{}", heading("Theme Preview", ui));
                println!("  {} success", status_symbol("ready", ui));
                println!("  {} warning", status_symbol("warn", ui));
                println!("  {} error", status_symbol("error", ui));
                println!("  {} info", status_symbol("info", ui));
                println!("  {} command", command_text("colab status", ui));
                Ok(())
            }
        }
    }
}

fn handle_settings_experiments(
    command: Option<SettingsExperimentsCommands>,
    ui: Ui,
    json: bool,
) -> Result<()> {
    match command {
        None => render_experiments(ui, json),
        Some(SettingsExperimentsCommands::Get { key: None }) => {
            let cfg = load_cocli_config()?;
            if json {
                print_value(true, &cfg.experiments)
            } else {
                for item in experiment_items(&cfg) {
                    println!(
                        "[{}] {:<28} {}",
                        if item.enabled { "x" } else { " " },
                        item.label,
                        muted(item.risk, ui)
                    );
                }
                Ok(())
            }
        }
        Some(SettingsExperimentsCommands::Get { key: Some(key) }) => handle_config_get_key(
            &format!("experiments.{}", experiment_config_key(&key)),
            json,
        ),
        Some(SettingsExperimentsCommands::Set { key, value }) => {
            handle_experiment_set(&normalize_experiment_key(&key), value, json)
        }
        Some(SettingsExperimentsCommands::Reset) => {
            let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
            let mut cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            cfg.experiments = config::ExperimentsConfig::default();
            cfg.save(&path)
                .map_err(|e| ColabError::config(e.to_string()))?;
            Ok(())
        }
    }
}

fn render_experiments(ui: Ui, json: bool) -> Result<()> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    let cfg = config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
    if json {
        return print_value(true, &cfg.experiments);
    }

    if ui.interactive {
        return run_settings_editor(path, cfg, SettingsPage::Experiments, ui);
    }

    println!("{}", heading("Experiments", ui));
    println!("Optional features are off by default");
    println!();
    println!("Config path");
    println!("  {}", path_text(&path.display().to_string(), ui));
    println!();
    for item in experiment_items(&cfg) {
        println!(
            "[{}] {:<28} {}",
            if item.enabled { "x" } else { " " },
            item.label,
            muted(item.risk, ui)
        );
    }
    Ok(())
}

struct ExperimentItem {
    label: &'static str,
    risk: &'static str,
    enabled: bool,
}

fn experiment_items(cfg: &config::CocliConfig) -> [ExperimentItem; 8] {
    [
        ExperimentItem {
            label: "Continue",
            risk: "checkpoint/replay; not live memory",
            enabled: cfg.experiments.continue_work,
        },
        ExperimentItem {
            label: "Distribute",
            risk: "recipes, pools, shards; no quota bypass",
            enabled: cfg.experiments.distribute,
        },
        ExperimentItem {
            label: "Multi-login",
            risk: "locked unless Distribute is on",
            enabled: cfg.experiments.multi_login,
        },
        ExperimentItem {
            label: "MCP server",
            risk: "stdio tool surface",
            enabled: cfg.experiments.mcp_server,
        },
        ExperimentItem {
            label: "AI plan runner",
            risk: "requires explicit plan and confirmation",
            enabled: cfg.experiments.ai_plan_runner,
        },
        ExperimentItem {
            label: "AST observer",
            risk: "local read-only code outline",
            enabled: cfg.experiments.ast_observer,
        },
        ExperimentItem {
            label: "Secrets bridge",
            risk: "pass local secrets into CLI-run code",
            enabled: cfg.experiments.secrets_bridge,
        },
        ExperimentItem {
            label: "Background live checks",
            risk: "may touch network",
            enabled: cfg.experiments.background_live_checks,
        },
    ]
}

fn normalize_experiment_key(key: &str) -> String {
    key.replace('-', "_")
}

fn experiment_config_key(key: &str) -> String {
    match normalize_experiment_key(key).as_str() {
        "continue" => "continue_work".to_string(),
        other => other.to_string(),
    }
}

fn handle_experiment_set(key: &str, value: String, json: bool) -> Result<()> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    let mut cfg =
        config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
    let enabled = parse_bool(&value)?;
    match key {
        "continue" | "continue_work" => cfg.experiments.continue_work = enabled,
        "distribute" => {
            cfg.experiments.distribute = enabled;
            cfg.experiments.fleet = enabled;
            if !enabled {
                cfg.experiments.multi_login = false;
            }
        }
        "multi_login" => {
            if enabled && !cfg.experiments.distribute {
                return Err(ColabError::config(
                    "multi-login requires distribute\nenable: colab settings experiments set distribute true",
                ));
            }
            cfg.experiments.multi_login = enabled;
        }
        "mcp_server" => cfg.experiments.mcp_server = enabled,
        "ai_plan_runner" => cfg.experiments.ai_plan_runner = enabled,
        "ast_observer" => cfg.experiments.ast_observer = enabled,
        "secrets_bridge" => cfg.experiments.secrets_bridge = enabled,
        "background_live_checks" => cfg.experiments.background_live_checks = enabled,
        "fleet" => {
            cfg.experiments.distribute = enabled;
            cfg.experiments.fleet = enabled;
        }
        "slurp_automation" => {
            cfg.experiments.distribute = enabled;
            cfg.experiments.slurp_automation = enabled;
        }
        _ => return Err(ColabError::config(format!("unknown experiment: {key}"))),
    }
    cfg.save(&path)
        .map_err(|e| ColabError::config(e.to_string()))?;
    if json {
        print_value(true, &serde_json::json!({ key: enabled }))
    } else {
        println!("{key} {}", on_off(enabled));
        Ok(())
    }
}

fn experiments_summary(cfg: &config::CocliConfig) -> String {
    let enabled = experiment_items(cfg)
        .into_iter()
        .filter(|item| item.enabled)
        .count();
    if enabled == 0 {
        "all off".to_string()
    } else {
        format!("{enabled} enabled")
    }
}

fn render_ui_settings(ui: Ui, json: bool) -> Result<()> {
    let path = config::config_path().map_err(|e| ColabError::config(e.to_string()))?;
    let cfg = config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
    if json {
        return print_value(true, &cfg.ui);
    }

    if ui.interactive {
        return run_settings_editor(path, cfg, SettingsPage::Ui, ui);
    }

    println!("{}", heading("UI settings", ui));
    println!("Changes are saved to config.toml");
    println!();
    println!("Config path");
    println!("  {}", path_text(&path.display().to_string(), ui));
    println!();
    for (index, item) in ui_items(&cfg).iter().enumerate() {
        let marker = if index == 0 { "›" } else { " " };
        println!(
            "{marker} [{}] {:<15} {}",
            if item.enabled { "x" } else { " " },
            item.label,
            muted(item.description, ui)
        );
    }
    Ok(())
}

struct UiItem {
    label: &'static str,
    description: &'static str,
    enabled: bool,
}

fn ui_items(cfg: &config::CocliConfig) -> [UiItem; 9] {
    [
        UiItem {
            label: "Colour",
            description: "Friendly colour by default",
            enabled: cfg.ui.color != config::ColorChoice::Never,
        },
        UiItem {
            label: "Animations",
            description: "Progress motion for interactive runs",
            enabled: cfg.ui.animations,
        },
        UiItem {
            label: "Terminal bell",
            description: "Optional bell after long jobs",
            enabled: cfg.ui.bell,
        },
        UiItem {
            label: "Fun lines",
            description: "Rare harmless lines in interactive mode",
            enabled: cfg.ui.fun,
        },
        UiItem {
            label: "Compact mode",
            description: "Less spacing, smaller tables",
            enabled: cfg.ui.compact,
        },
        UiItem {
            label: "Icons",
            description: "Small symbols in human output",
            enabled: cfg.ui.icons,
        },
        UiItem {
            label: "Unicode",
            description: "Smooth borders and glyphs",
            enabled: cfg.ui.unicode,
        },
        UiItem {
            label: "TUI always",
            description: "Prefer panels whenever possible",
            enabled: cfg.ui.tui == "always",
        },
        UiItem {
            label: "TUI never",
            description: "Disable interactive panels",
            enabled: cfg.ui.tui == "never",
        },
    ]
}

fn normalize_ui_key(key: &str) -> String {
    match key {
        "colour" => "color".to_string(),
        other => other.to_string(),
    }
}

fn handle_settings_support(command: SupportCommands, json: bool) -> Result<()> {
    match command {
        SupportCommands::BugReport { show_private } => handle_bug_report(show_private, json),
        SupportCommands::Redact { text } => {
            let mut input = text.unwrap_or_default();
            if input.is_empty() {
                std::io::stdin().read_to_string(&mut input)?;
            }
            println!(
                "{}",
                crate::cocli::auth::redaction::redact_sensitive(&input)
            );
            Ok(())
        }
        SupportCommands::Bundle => handle_bug_report(false, json),
    }
}

fn color_choice_name(color: config::ColorChoice) -> &'static str {
    match color {
        config::ColorChoice::Always => "always",
        config::ColorChoice::Auto => "auto",
        config::ColorChoice::Never => "never",
    }
}

fn maintainer_allowed() -> bool {
    if std::env::var_os("COLAB_CLI_MAINTAINER").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return true;
    }
    let allowed = std::env::var("COLAB_CLI_OWNER").unwrap_or_else(|_| "keys".to_string());
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    if user == allowed {
        return true;
    }
    git_identity_is_maintainer()
}

fn dev_visible(cfg: &config::CocliConfig) -> bool {
    maintainer_allowed() && (cfg.dev.enabled || std::env::var_os("COLAB_CLI_DEV").is_some())
}

#[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
fn dev_tools_unlocked() -> Result<bool> {
    let cfg = load_cocli_config()?;
    Ok(dev_visible(&cfg))
}

fn git_identity_is_maintainer() -> bool {
    let email = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default();
    email.ends_with("@users.noreply.github.com") && email.contains("keys-i")
}

fn handle_skills(cmd: SkillCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        SkillCommands::List {
            json: local_json,
            category,
            scope,
            risk,
            needs_session,
            enabled: _enabled,
            disabled,
        } => {
            let category = scope.or(category);
            let rows = if disabled {
                Vec::new()
            } else {
                skill_rows(category.as_deref(), risk.as_deref(), needs_session)
            };
            if json || local_json {
                print_value(true, &rows)
            } else {
                print_skill_catalog(
                    "Skills",
                    "Agent-friendly tools and optional integrations",
                    &rows,
                    ui,
                )
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
                println!("{}", heading(&format!("Skill {}", row.name), ui));
                println!("  summary         {}", row.summary);
                println!("  scope           {}", row.scope);
                println!("  risk            {}", row.risk);
                println!("  session         {}", yes_no(row.needs_session));
                println!("  network         {}", yes_no(row.network));
                println!("  inputs          {}", row.inputs.join(", "));
                println!("  outputs         {}", row.outputs.join(", "));
                println!("  examples");
                for example in &row.examples {
                    println!("    {}", command_text(example, ui));
                }
                println!("  safety notes");
                for note in &row.safety_notes {
                    println!("    {}", muted(note, ui));
                }
                println!("  json schema     {}", row.json_schema);
                Ok(())
            }
        }
        SkillCommands::Run {
            name,
            input_json,
            yes,
        } => {
            let input: serde_json::Value = serde_json::from_str(&input_json)?;
            let output = run_skill_plan(&name, input, yes)?;
            print_value(true, &output)
        }
        SkillCommands::Enable { name } | SkillCommands::Disable { name } => {
            Err(ColabError::config(format!(
                "skill toggles are not implemented; built-in skill `{name}` is always available"
            )))
        }
        SkillCommands::Mcp { stdio } => {
            let data = serde_json::json!({
                "stdio": stdio,
                "tools": skill_rows(None, None, false).into_iter().map(|row| row.name).collect::<Vec<_>>(),
                "note": "MCP stdio is a stable JSON tool listing here; transport server is not started by default"
            });
            print_value(json, &data)
        }
    }
}

fn print_skill_catalog(title: &str, subtitle: &str, rows: &[SkillRow], ui: Ui) -> Result<()> {
    println!("{}", heading(title, ui));
    println!("{subtitle}");
    println!();
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            vec![
                command_text(row.name, ui),
                skill_value("risk", row.risk, ui),
                skill_value("session", yes_no(row.needs_session), ui),
                skill_value("network", yes_no(row.network), ui),
                skill_value("state", row.state, ui),
                row.summary.to_string(),
            ]
        })
        .collect();
    let headers = ["Tool", "Risk", "Session", "Network", "State", "Summary"]
        .into_iter()
        .map(|h| table_header(h, ui))
        .collect::<Vec<_>>();
    let header_refs = headers.iter().map(String::as_str).collect::<Vec<_>>();
    print!(
        "{}",
        crate::cocli::ui::table::render_table(
            &header_refs,
            &table_rows,
            crate::cocli::ui::width::terminal_width()
        )
    );
    Ok(())
}

fn table_header(text: &str, ui: Ui) -> String {
    if ui.plain {
        text.to_string()
    } else {
        text.bright_cyan().bold().to_string()
    }
}

fn skill_value(kind: &str, value: &str, ui: Ui) -> String {
    if ui.plain {
        return value.to_string();
    }
    match (kind, value) {
        ("risk", "low") => value.bright_green().to_string(),
        ("risk", "med") => value.yellow().to_string(),
        ("risk", "high") => value.bright_red().to_string(),
        ("session", "yes") => value.bright_magenta().to_string(),
        ("network", "yes") => value.bright_blue().to_string(),
        ("state", "ready") => value.bright_green().to_string(),
        ("state", "gated") => value.yellow().to_string(),
        ("state", "off") => value.dimmed().to_string(),
        _ => value.to_string(),
    }
}

#[derive(Clone, serde::Serialize)]
struct SkillRow {
    name: &'static str,
    scope: &'static str,
    category: &'static str,
    risk: &'static str,
    needs_session: bool,
    network: bool,
    state: &'static str,
    summary: &'static str,
    inputs: Vec<&'static str>,
    outputs: Vec<&'static str>,
    examples: Vec<&'static str>,
    safety_notes: Vec<&'static str>,
    json_schema: serde_json::Value,
}

fn skill_rows(category: Option<&str>, risk: Option<&str>, needs_session: bool) -> Vec<SkillRow> {
    agent_skill_rows()
        .into_iter()
        .filter(|row| category.is_none_or(|want| row.category == want))
        .filter(|row| risk.is_none_or(|want| row.risk == want))
        .filter(|row| !needs_session || row.needs_session)
        .collect()
}

fn agent_skill_rows() -> Vec<SkillRow> {
    let cfg = load_cocli_config().unwrap_or_default();
    let mut rows = vec![
        skill(
            "recipe.plan",
            "workflow",
            "low",
            false,
            false,
            "Explain a recipe plan",
            &["config"],
            &["plan", "findings"],
            &["colab distribute recipe explain --json"],
            &["Local read only"],
        ),
        skill(
            "recipe.explain",
            "workflow",
            "low",
            false,
            false,
            "Render a clean recipe explanation",
            &["config"],
            &["summary"],
            &["colab distribute recipe explain --json"],
            &["Local read only"],
        ),
        skill(
            "distribute.plan",
            "distribute",
            "med",
            false,
            false,
            "Plan approved runtime work",
            &["config", "cost"],
            &["plan", "compliance"],
            &["colab distribute plan --json"],
            &["No quota bypass", "No hidden execution"],
        ),
        skill(
            "distribute.status",
            "distribute",
            "low",
            false,
            false,
            "Show distribute planning status",
            &["config"],
            &["status"],
            &["colab distribute status --json"],
            &["Local read only"],
        ),
        skill(
            "continue.save",
            "state",
            "low",
            true,
            false,
            "Save checkpoint metadata",
            &["session", "name", "artifacts"],
            &["manifest"],
            &["colab continue save --session work --name run-a"],
            &["Does not copy live Python memory"],
        ),
        skill(
            "continue.resume",
            "state",
            "med",
            false,
            true,
            "Resume from checkpoint metadata",
            &["name", "dry_run"],
            &["resume_plan"],
            &["colab continue resume run-a --dry-run --json"],
            &["Network work requires explicit command flags"],
        ),
        skill(
            "runtime.inspect",
            "runtime",
            "low",
            true,
            true,
            "Inspect runtime metadata",
            &["session"],
            &["runtime"],
            &["colab status runtime --all --json"],
            &["No package installs"],
        ),
        skill(
            "fs.diff",
            "files",
            "low",
            true,
            true,
            "Compare local and remote trees",
            &["local", "remote"],
            &["diff"],
            &["colab fs diff ./src /content/src --json"],
            &["Destructive sync requires separate confirmation"],
        ),
        skill(
            "fs.changed",
            "files",
            "low",
            true,
            true,
            "Show local changes that sync would upload",
            &["local", "remote"],
            &["changed"],
            &["colab fs changed ./src /content/src --json"],
            &["Read-only comparison"],
        ),
        skill(
            "support.bug-report",
            "support",
            "low",
            false,
            false,
            "Write a redacted diagnostic bundle",
            &["show_private"],
            &["bundle"],
            &["colab settings support bug-report --json"],
            &["Secrets are redacted by default"],
        ),
        skill(
            "mcp.tools",
            "agent",
            "low",
            false,
            false,
            "List MCP-compatible tool metadata",
            &[],
            &["tools"],
            &["colab settings skills mcp --json"],
            &["No transport server starts unless requested"],
        ),
        skill(
            "ast.outline",
            "code",
            "low",
            false,
            false,
            "Outline local code",
            &["file"],
            &["imports", "functions", "classes"],
            &["colab run ast file.py --json"],
            &["Local read only"],
        ),
        skill(
            "ast.watch",
            "code",
            "low",
            false,
            false,
            "Watch a local code outline",
            &["file"],
            &["outline"],
            &["colab run watch file.py --ast"],
            &["Local read only"],
        ),
        skill(
            "mcp.invoke",
            "agent",
            "med",
            false,
            false,
            "Plan a built-in tool invocation from JSON",
            &["name", "input"],
            &["planned_command"],
            &["colab settings skills run recipe.plan --json-input '{}'"],
            &["Plans are inspectable before execution"],
        ),
        skill(
            "agent.plan",
            "agent",
            "low",
            false,
            false,
            "Draft an explicit agent plan",
            &["goal"],
            &["plan"],
            &["colab settings skills inspect agent.plan"],
            &["No autonomous execution"],
        ),
        skill(
            "agent.audit",
            "agent",
            "low",
            false,
            false,
            "Check a plan before running it",
            &["plan"],
            &["findings"],
            &["colab settings skills inspect agent.audit"],
            &["Destructive actions require confirmation"],
        ),
        skill(
            "secret.inject",
            "secrets",
            "med",
            false,
            false,
            "Request named secrets for a run",
            &["keys"],
            &["redacted_request"],
            &["colab run script train.py --env HF_TOKEN --json"],
            &["Values are never exposed through the tool catalog"],
        ),
        skill(
            "run.with_env",
            "secrets",
            "med",
            true,
            true,
            "Run code with explicit local secret env",
            &["command", "keys"],
            &["execution"],
            &["colab run py --env HF_TOKEN --code '...'"],
            &["No hidden environment forwarding"],
        ),
    ];
    rows.extend([
        skill(
            "kernel.list",
            "kernel",
            "low",
            true,
            true,
            "List running kernels",
            &["session"],
            &["kernels"],
            &["colab session kernel list --json"],
            &["Read-only"],
        ),
        skill(
            "kernel.select",
            "kernel",
            "low",
            true,
            true,
            "Select the active kernel",
            &["kernel"],
            &["selection"],
            &["colab session kernel select python3"],
            &["No code execution"],
        ),
        skill(
            "kernel.restart",
            "kernel",
            "med",
            true,
            true,
            "Restart the selected kernel",
            &["yes"],
            &["status"],
            &["colab session kernel restart --yes --json"],
            &["Loses in-kernel variables"],
        ),
        skill(
            "kernel.interrupt",
            "kernel",
            "low",
            true,
            true,
            "Interrupt running kernel code",
            &["yes"],
            &["status"],
            &["colab session kernel interrupt --json"],
            &["Stops current execution where supported"],
        ),
    ]);
    if let Some(info) = cached_kernel_language() {
        match info.language {
            KernelLanguage::Python => rows.push(skill(
                "pkg.python",
                "kernel",
                "low",
                true,
                true,
                "Run Python package tooling",
                &["packages"],
                &["pip_output"],
                &["colab run pkg add numpy --json"],
                &["Routes through active Python kernel"],
            )),
            KernelLanguage::Julia => rows.push(skill(
                "pkg.julia",
                "kernel",
                "low",
                true,
                true,
                "Run Julia Pkg tooling",
                &["packages"],
                &["pkg_output"],
                &["colab run pkg add CSV --json"],
                &["Routes through active Julia kernel"],
            )),
            KernelLanguage::R => rows.push(skill(
                "pkg.r",
                "kernel",
                "low",
                true,
                true,
                "Run R package tooling",
                &["packages"],
                &["package_output"],
                &["colab run pkg add dplyr --json"],
                &["Routes through active R kernel"],
            )),
            _ => {}
        }
    }
    rows.retain(|row| {
        if row.name.starts_with("recipe.") || row.name.starts_with("distribute.") {
            cfg.experiments.distribute
        } else if row.name.starts_with("continue.") {
            cfg.experiments.continue_work
        } else if row.name.starts_with("ast.") {
            cfg.experiments.ast_observer
        } else if row.name.starts_with("mcp.") {
            cfg.experiments.mcp_server
        } else if row.name.starts_with("secret.") || row.name == "run.with_env" {
            cfg.experiments.secrets_bridge
        } else {
            true
        }
    });
    rows
}

#[allow(clippy::too_many_arguments)]
fn skill(
    name: &'static str,
    scope: &'static str,
    risk: &'static str,
    needs_session: bool,
    network: bool,
    summary: &'static str,
    inputs: &[&'static str],
    outputs: &[&'static str],
    examples: &[&'static str],
    safety_notes: &[&'static str],
) -> SkillRow {
    SkillRow {
        name,
        scope,
        category: scope,
        risk,
        needs_session,
        network,
        state: "ready",
        summary,
        inputs: inputs.to_vec(),
        outputs: outputs.to_vec(),
        examples: examples.to_vec(),
        safety_notes: safety_notes.to_vec(),
        json_schema: serde_json::json!({
            "type": "object",
            "additionalProperties": true
        }),
    }
}

fn run_skill_plan(
    name: &str,
    input: serde_json::Value,
    yes: bool,
) -> Result<crate::cocli::r#continue::manifest::ToolOutput> {
    if name == "mcp.tools" {
        return Ok(crate::cocli::r#continue::manifest::ToolOutput {
            tool: name.to_string(),
            status: "planned".to_string(),
            data: serde_json::json!({ "tools": skill_rows(None, None, false) }),
            audit: vec!["listed skill catalog".to_string()],
        });
    }
    if name == "mcp.invoke" {
        let tool = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ColabError::config("mcp.invoke needs input.name"))?;
        let nested = input
            .get("input")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        return run_skill_plan(tool, nested, yes);
    }
    crate::cocli::tools::ToolRegistry::run_plan(name, input, yes)
        .map_err(|e| ColabError::config(e.to_string()))
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn muted(text: &str, ui: Ui) -> String {
    if ui.plain {
        text.to_string()
    } else {
        text.dimmed().to_string()
    }
}

fn handle_distribute(cmd: Option<DistributeCommands>, ui: Ui, json: bool) -> Result<()> {
    require_experiment("distribute", |cfg| cfg.experiments.distribute)?;
    match cmd {
        None => {
            println!("{}", heading("Distribute", ui));
            println!("Experimental recipes, pools, and shards");
            println!();
            println!("  recipe      tiny TOML workflow config");
            println!("  pool        approved runtime pool planning");
            println!("  shard       split work into safe chunks");
            Ok(())
        }
        Some(DistributeCommands::Plan(args)) => {
            handle_fleet(FleetCommands::Plan(recipe_args(args)), ui, json)
        }
        Some(DistributeCommands::Status { config }) => {
            let path = recipe_config(config);
            let recipe_found = Path::new(&path).exists();
            let multi_login = load_cocli_config()
                .map(|cfg| cfg.experiments.multi_login)
                .unwrap_or(false);
            if json {
                return print_value(
                    true,
                    &serde_json::json!({
                    "enabled": true,
                    "recipe": path,
                    "recipe_found": recipe_found,
                    "multi_login": multi_login
                    }),
                );
            }
            println!("{}", heading("Distribute status", ui));
            println!("  enabled         yes");
            println!("  recipe          {}", path_text(&path, ui));
            println!("  recipe found    {}", yes_no(recipe_found));
            println!("  multi-login     {}", on_off(multi_login));
            Ok(())
        }
        Some(DistributeCommands::Explain(args)) => {
            handle_slurp(SlurpCommands::Explain(recipe_args(args)), ui, json)
        }
        Some(DistributeCommands::Run(args)) => {
            if !args.dry_run && !args.confirm {
                return Err(ColabError::config(
                    "distribute run requires --dry-run or --confirm",
                ));
            }
            handle_fleet(FleetCommands::Exec(recipe_run_args(args)), ui, json)
        }
        Some(DistributeCommands::Resume(args)) => {
            handle_fleet(FleetCommands::Exec(recipe_args(args)), ui, json)
        }
        Some(DistributeCommands::Clean) => {
            print_value(json, &serde_json::json!({ "ok": true, "cleaned": 0 }))
        }
        Some(DistributeCommands::Recipe { command }) => match command {
            DistributeRecipeCommands::Init { out } => {
                handle_slurp(SlurpCommands::Init { out }, ui, json)
            }
            DistributeRecipeCommands::Check(args) => {
                handle_slurp(SlurpCommands::Check(recipe_args(args)), ui, json)
            }
            DistributeRecipeCommands::Explain(args) => {
                handle_slurp(SlurpCommands::Explain(recipe_args(args)), ui, json)
            }
            DistributeRecipeCommands::Run(args) => {
                if !args.dry_run && !args.confirm {
                    return Err(ColabError::config(
                        "distribute recipe run requires --dry-run or --confirm",
                    ));
                }
                handle_fleet(FleetCommands::Exec(recipe_run_args(args)), ui, json)
            }
        },
        Some(DistributeCommands::Pool { command }) => match command {
            DistributePoolCommands::Plan(args) => {
                handle_fleet(FleetCommands::Plan(recipe_args(args)), ui, json)
            }
            DistributePoolCommands::Status { config } => {
                let args = FleetConfigArgs {
                    config,
                    dry_run: true,
                    cost: false,
                    allow_fallback_account: false,
                };
                handle_fleet(FleetCommands::Plan(recipe_args(args)), ui, json)
            }
            DistributePoolCommands::Cost(mut args) => {
                args.cost = true;
                handle_fleet(FleetCommands::Plan(recipe_args(args)), ui, json)
            }
            DistributePoolCommands::Logs => print_value(json, &serde_json::json!({ "logs": [] })),
        },
        Some(DistributeCommands::Shard { command }) => match command {
            DistributeShardCommands::Plan(args) => {
                handle_fleet(FleetCommands::Plan(recipe_args(args)), ui, json)
            }
            DistributeShardCommands::Run(args) => {
                if !args.dry_run && !args.confirm {
                    return Err(ColabError::config(
                        "distribute shard run requires --dry-run or --confirm",
                    ));
                }
                handle_fleet(FleetCommands::Exec(recipe_run_args(args)), ui, json)
            }
            DistributeShardCommands::Resume(args) => {
                handle_fleet(FleetCommands::Exec(recipe_args(args)), ui, json)
            }
        },
    }
}

fn recipe_args(mut args: FleetConfigArgs) -> FleetConfigArgs {
    args.config = recipe_config(args.config);
    args
}

fn recipe_run_args(args: DistributeRunArgs) -> FleetConfigArgs {
    FleetConfigArgs {
        config: recipe_config(args.config),
        dry_run: args.dry_run,
        cost: args.cost,
        allow_fallback_account: args.allow_fallback_account,
    }
}

fn recipe_config(config: String) -> String {
    if config == "colab.recipe.toml"
        && !Path::new(&config).exists()
        && Path::new("slurp.toml").exists()
    {
        "slurp.toml".to_string()
    } else {
        config
    }
}

fn handle_fleet(cmd: FleetCommands, ui: Ui, json: bool) -> Result<()> {
    require_experiment("distribute", |cfg| cfg.experiments.distribute)?;
    match cmd {
        FleetCommands::Plan(args) => {
            let (cfg, plan, findings) = fleet_plan_from_args(&args)?;
            print_fleet_plan(&cfg, &plan, &findings, json)
        }
        FleetCommands::Start(args) | FleetCommands::Exec(args) => {
            let (cfg, plan, findings) = fleet_plan_from_args(&args)?;
            refuse_if_needed(&findings)?;
            if args.dry_run {
                return print_fleet_plan(&cfg, &plan, &findings, json || args.cost);
            }
            Err(ColabError::config(
                "distribute execution is deferred; run `colab distribute plan --cost`",
            ))
        }
        FleetCommands::Doctor => {
            migration(&ui, "colab status fleet");
            let data = serde_json::json!({
                "distribute_mode": "compliant",
                "fallback_rotation": false,
                "next_action": "run `colab distribute plan --config colab.recipe.toml`"
            });
            print_value(json, &data)
        }
    }
}

fn handle_slurp(cmd: SlurpCommands, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        SlurpCommands::Init { out } => {
            if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                print!("Recipe name [llama-batch-run]: ");
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
                println!("recipe written");
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
            require_experiment("distribute", |cfg| cfg.experiments.distribute)?;
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
            migration(&ui, "colab status slurp");
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

#[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
fn handle_release(cmd: ReleaseCommands, json: bool) -> Result<()> {
    match cmd {
        ReleaseCommands::Name { version } => {
            let version = version.unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
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

#[derive(Debug, serde::Serialize)]
struct CodeOutline {
    file: String,
    kind: String,
    imports: Vec<String>,
    functions: Vec<String>,
    classes: Vec<String>,
    cells: usize,
    main_guard: bool,
    top_level_calls: Vec<String>,
    shell_escapes: Vec<String>,
    deps: Vec<String>,
}

fn print_code_outline(path: &str, json: bool) -> Result<()> {
    let outline = code_outline(path)?;
    if json {
        return print_value(true, &outline);
    }
    println!("{}", heading("AST outline", Ui::new(false, false, false)));
    println!("  file            {}", outline.file);
    println!("  kind            {}", outline.kind);
    if outline.kind.ends_with("-basic") {
        println!("  parser          basic outline");
    }
    if outline.cells > 0 {
        println!("  notebook cells  {}", outline.cells);
    }
    print_outline_list("imports", &outline.imports);
    print_outline_list("classes", &outline.classes);
    print_outline_list("functions", &outline.functions);
    print_outline_list("top calls", &outline.top_level_calls);
    print_outline_list("shell escapes", &outline.shell_escapes);
    print_outline_list("deps", &outline.deps);
    println!("  main guard      {}", yes_no(outline.main_guard));
    Ok(())
}

fn print_outline_list(label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("  {label:<14} {}", values.join(", "));
}

fn code_outline(path: &str) -> Result<CodeOutline> {
    let body = std::fs::read_to_string(path)?;
    if path.ends_with(".ipynb") {
        let value: serde_json::Value = serde_json::from_str(&body)?;
        let cells = value
            .get("cells")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut code = String::new();
        let mut count = 0usize;
        for cell in cells {
            if cell.get("cell_type").and_then(serde_json::Value::as_str) == Some("code") {
                count += 1;
                if let Some(source) = cell.get("source") {
                    if let Some(lines) = source.as_array() {
                        for line in lines {
                            code.push_str(line.as_str().unwrap_or_default());
                        }
                    } else if let Some(source) = source.as_str() {
                        code.push_str(source);
                    }
                }
                code.push('\n');
            }
        }
        let mut outline = python_outline(path, &code);
        outline.kind = "notebook".to_string();
        outline.cells = count;
        Ok(outline)
    } else if path.ends_with(".jl") {
        Ok(julia_outline(path, &body))
    } else if path.ends_with(".R") || path.ends_with(".r") {
        Ok(r_outline(path, &body))
    } else {
        Ok(python_outline(path, &body))
    }
}

fn python_outline(path: &str, body: &str) -> CodeOutline {
    let mut outline = CodeOutline {
        file: path.to_string(),
        kind: "python".to_string(),
        imports: Vec::new(),
        functions: Vec::new(),
        classes: Vec::new(),
        cells: 0,
        main_guard: false,
        top_level_calls: Vec::new(),
        shell_escapes: Vec::new(),
        deps: Vec::new(),
    };
    for line in body.lines() {
        let trimmed = line.trim();
        let indent = line.len().saturating_sub(line.trim_start().len());
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or(rest)
                .trim_end_matches(',');
            push_unique(&mut outline.imports, name);
            push_unique(&mut outline.deps, name.split('.').next().unwrap_or(name));
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            let name = rest.split_whitespace().next().unwrap_or(rest);
            push_unique(&mut outline.imports, name);
            push_unique(&mut outline.deps, name.split('.').next().unwrap_or(name));
        } else if let Some(rest) = trimmed.strip_prefix("def ") {
            push_unique(
                &mut outline.functions,
                rest.split('(').next().unwrap_or(rest),
            );
        } else if let Some(rest) = trimmed.strip_prefix("class ") {
            push_unique(
                &mut outline.classes,
                rest.split(['(', ':']).next().unwrap_or(rest),
            );
        } else if trimmed.contains("__name__") && trimmed.contains("__main__") {
            outline.main_guard = true;
        } else if indent == 0 && trimmed.ends_with(')') && !trimmed.starts_with('#') {
            push_unique(&mut outline.top_level_calls, trimmed);
        }
        if trimmed.starts_with('!')
            || trimmed.contains("os.system(")
            || trimmed.contains("subprocess.")
        {
            push_unique(&mut outline.shell_escapes, trimmed);
        }
    }
    outline
}

fn julia_outline(path: &str, body: &str) -> CodeOutline {
    let mut outline = CodeOutline {
        file: path.to_string(),
        kind: "julia-basic".to_string(),
        imports: Vec::new(),
        functions: Vec::new(),
        classes: Vec::new(),
        cells: 0,
        main_guard: false,
        top_level_calls: Vec::new(),
        shell_escapes: Vec::new(),
        deps: Vec::new(),
    };
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("using ")
            .or_else(|| trimmed.strip_prefix("import "))
        {
            for name in rest.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                push_unique(&mut outline.imports, name);
                push_unique(&mut outline.deps, name.split('.').next().unwrap_or(name));
            }
        } else if let Some(rest) = trimmed.strip_prefix("function ") {
            push_unique(
                &mut outline.functions,
                rest.split('(').next().unwrap_or(rest),
            );
        } else if let Some(rest) = trimmed.strip_prefix("struct ") {
            push_unique(
                &mut outline.classes,
                rest.split_whitespace().next().unwrap_or(rest),
            );
        } else if let Some(rest) = trimmed.strip_prefix("module ") {
            push_unique(
                &mut outline.classes,
                rest.split_whitespace().next().unwrap_or(rest),
            );
        } else if let Some(rest) = trimmed.strip_prefix("macro ") {
            push_unique(
                &mut outline.functions,
                rest.split('(').next().unwrap_or(rest),
            );
        }
        if trimmed.starts_with(';') || trimmed.contains("run(`") {
            push_unique(&mut outline.shell_escapes, trimmed);
        }
    }
    outline
}

fn r_outline(path: &str, body: &str) -> CodeOutline {
    let mut outline = CodeOutline {
        file: path.to_string(),
        kind: "r-basic".to_string(),
        imports: Vec::new(),
        functions: Vec::new(),
        classes: Vec::new(),
        cells: 0,
        main_guard: false,
        top_level_calls: Vec::new(),
        shell_escapes: Vec::new(),
        deps: Vec::new(),
    };
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed
            .strip_prefix("library(")
            .or_else(|| trimmed.strip_prefix("require("))
            .and_then(|rest| rest.split(')').next())
        {
            let name = name.trim_matches(['"', '\'']);
            push_unique(&mut outline.imports, name);
            push_unique(&mut outline.deps, name);
        } else if trimmed.contains("function(") {
            let name = trimmed
                .split("<-")
                .next()
                .or_else(|| trimmed.split('=').next())
                .unwrap_or("anonymous")
                .trim();
            push_unique(&mut outline.functions, name);
        } else if trimmed.starts_with("source(") {
            push_unique(&mut outline.top_level_calls, trimmed);
        }
        if trimmed.contains("system(") || trimmed.contains("shell(") {
            push_unique(&mut outline.shell_escapes, trimmed);
        }
    }
    outline
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if !value.is_empty() && !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn handle_ai(cmd: Option<AiCommands>, ui: Ui, json: bool) -> Result<()> {
    match cmd {
        None => {
            println!("{}", heading("AI", ui));
            println!("Agent, MCP, and code tools");
            println!();
            println!("  tools       list and inspect agent-friendly tools");
            println!("  plan        draft an inspectable plan");
            println!("  audit       check a saved plan");
            println!("  mcp         disabled unless enabled in experiments");
            Ok(())
        }
        Some(AiCommands::Tools { command }) => handle_ai_tools(command, ui, json),
        Some(AiCommands::Mcp { command }) => handle_ai_mcp(command, json),
        Some(AiCommands::Plan { goal, out }) => {
            require_experiment("ai plan runner", |cfg| cfg.experiments.ai_plan_runner)?;
            let plan = format!(
                "goal = {goal:?}\nconfirm_required = true\n\n[[steps]]\ntool = \"ai.audit\"\ninput = {{}}\n"
            );
            if let Some(out) = out {
                std::fs::write(&out, plan)?;
                ui.success(&format!("plan written: {out}"));
            } else {
                print!("{plan}");
            }
            Ok(())
        }
        Some(AiCommands::Audit { plan_file }) => {
            let body = std::fs::read_to_string(&plan_file)?;
            print_value(
                json,
                &serde_json::json!({
                    "plan": plan_file,
                    "bytes": body.len(),
                    "confirm_required": !body.contains("confirm_required = false"),
                }),
            )
        }
        Some(AiCommands::Explain { plan_file }) => {
            let body = std::fs::read_to_string(&plan_file)?;
            if json {
                print_value(
                    true,
                    &serde_json::json!({ "plan": plan_file, "text": body }),
                )
            } else {
                println!("{}", heading("AI plan", ui));
                println!("{}", body.trim());
                Ok(())
            }
        }
        Some(AiCommands::Run { plan_file, confirm }) => {
            require_experiment("ai plan runner", |cfg| cfg.experiments.ai_plan_runner)?;
            if !confirm {
                return Err(ColabError::config("ai run requires --confirm"));
            }
            let body = std::fs::read_to_string(&plan_file)?;
            append_audit(&format!("ai_run plan={plan_file} bytes={}", body.len()))?;
            ui.success("AI plan accepted for confirmed execution audit");
            Ok(())
        }
        Some(AiCommands::Ast {
            first,
            second,
            json: local_json,
        }) => {
            require_experiment("ast observer", |cfg| cfg.experiments.ast_observer)?;
            let file = if first == "watch" {
                second.ok_or_else(|| ColabError::config("ai ast watch needs a file"))?
            } else {
                first
            };
            print_code_outline(&file, json || local_json)
        }
        Some(AiCommands::Code { command }) => match command {
            AiCodeCommands::Explain {
                file,
                json: local_json,
            } => {
                require_experiment("ast observer", |cfg| cfg.experiments.ast_observer)?;
                print_code_outline(&file, json || local_json)
            }
            AiCodeCommands::Deps {
                file,
                json: local_json,
            } => {
                require_experiment("ast observer", |cfg| cfg.experiments.ast_observer)?;
                let outline = code_outline(&file)?;
                print_value(json || local_json, &outline.deps)
            }
        },
    }
}

fn handle_secret(cmd: SecretCommands, ui: Ui, json: bool) -> Result<()> {
    require_experiment("secrets bridge", |cfg| cfg.experiments.secrets_bridge)?;
    match cmd {
        SecretCommands::List => {
            if json {
                print_value(true, &serde_json::json!({ "secrets": [] }))
            } else {
                println!("{}", heading("Secrets", ui));
                println!("No persistent secret store is configured.");
                println!("fix: pass secrets with --env KEY, --env-file PATH, or --secret KEY");
                Ok(())
            }
        }
        SecretCommands::Set {
            key,
            from_env,
            prompt,
            value,
        } => {
            secrets::validate_key(&key)?;
            if value.is_some() && !ui.quiet {
                eprintln!(
                    "warning: passing secrets as CLI arguments can leak through shell history; prefer --from-env or --prompt"
                );
            }
            if let Some(local) = from_env.as_deref() {
                secrets::validate_key(local)?;
                if std::env::var_os(local).is_none() {
                    return Err(ColabError::config(format!(
                        "Missing secret: {local}\nfix: export {local}=..."
                    )));
                }
            }
            if prompt || from_env.is_some() || value.is_some() {
                if json {
                    print_value(
                        true,
                        &serde_json::json!({
                            "ok": true,
                            "stored": false,
                            "key": key,
                            "value": "<redacted>",
                            "message": "plaintext config storage is disabled; use --env or --env-file for run commands"
                        }),
                    )
                } else {
                    println!("secret validated: {key}");
                    println!("stored: no");
                    println!("fix: run with --env {key} or --secret {key}");
                    Ok(())
                }
            } else {
                Err(ColabError::config(
                    "secret set needs --from-env LOCAL_ENV, --prompt, or explicit --value",
                ))
            }
        }
        SecretCommands::Unset { key } => {
            secrets::validate_key(&key)?;
            if json {
                print_value(
                    true,
                    &serde_json::json!({ "ok": true, "removed": false, "key": key }),
                )
            } else {
                println!("secret not stored locally: {key}");
                Ok(())
            }
        }
        SecretCommands::Inject { keys } => {
            for key in &keys {
                secrets::validate_key(key)?;
            }
            print_value(
                json,
                &serde_json::json!({
                    "ok": true,
                    "keys": keys,
                    "values": "<redacted>",
                    "run_flag": "--env KEY"
                }),
            )
        }
        SecretCommands::Status | SecretCommands::Doctor => {
            let cfg = load_cocli_config()?;
            print_value_or_kv(
                json,
                "secrets",
                &serde_json::json!({
                    "experiment": cfg.experiments.secrets_bridge,
                    "provider": cfg.secrets.provider,
                    "plaintext_config": false,
                    "env": cfg.secrets.allow_env,
                    "env_file": cfg.secrets.allow_env_file,
                }),
            )
        }
        SecretCommands::ExportRedacted => print_value(
            json,
            &serde_json::json!({
                "secrets": [],
                "values": "<redacted>",
                "plaintext_config": false
            }),
        ),
    }
}

fn handle_ai_tools(cmd: Option<AiToolsCommands>, ui: Ui, json: bool) -> Result<()> {
    match cmd.unwrap_or(AiToolsCommands::List { json: false }) {
        AiToolsCommands::List { json: local_json } => {
            let rows = skill_rows(None, None, false);
            if json || local_json {
                print_value(true, &rows)
            } else {
                print_skill_catalog("AI tools", "Agent-facing workflows", &rows, ui)
            }
        }
        AiToolsCommands::Inspect {
            name,
            json: local_json,
        } => handle_skills(
            SkillCommands::Inspect {
                name,
                json: local_json,
            },
            ui,
            json,
        ),
    }
}

fn handle_ai_mcp(cmd: Option<AiMcpCommands>, json: bool) -> Result<()> {
    require_experiment("mcp server", |cfg| cfg.experiments.mcp_server)?;
    match cmd.unwrap_or(AiMcpCommands::Tools) {
        AiMcpCommands::Tools => {
            let rows = skill_rows(None, None, false);
            print_value(json, &serde_json::json!({ "tools": rows }))
        }
        AiMcpCommands::Serve { stdio: _ } => {
            Err(ColabError::config("MCP server not implemented yet"))
        }
    }
}

fn require_experiment(
    name: &str,
    enabled: impl FnOnce(&config::CocliConfig) -> bool,
) -> Result<()> {
    let cfg = load_cocli_config()?;
    let is_enabled = enabled(&cfg);
    debug::debug1(format!(
        "feature gate checked name={name} enabled={is_enabled} source=config"
    ));
    if is_enabled {
        Ok(())
    } else {
        Err(experiment_error(name))
    }
}

fn experiment_error(name: &str) -> ColabError {
    ColabError::config(format!(
        "experimental feature disabled: {name}\nenable: colab settings experiments"
    ))
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
            let session =
                session.ok_or_else(|| ColabError::config("continue save needs --session"))?;
            let name = name.ok_or_else(|| ColabError::config("continue save needs --name"))?;
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
                println!("colab > fast path found");
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
            confirm,
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
            if !confirm {
                return Err(ColabError::config(
                    "continue resume requires --dry-run or --confirm",
                ));
            }

            if new_runtime {
                handle_assign(
                    config,
                    ui,
                    AssignOptions {
                        variant: Some(if gpu.is_some() {
                            Variant::Gpu
                        } else {
                            Variant::Cpu
                        }),
                        accelerator: gpu,
                        name: Some(manifest.session.name.clone()),
                        shape: Shape::Standard,
                        keepalive: false,
                        retries: 1,
                    },
                )
                .await?;
            }

            for step in &steps {
                let no_secrets = SecretBundle::default();
                handle_run(
                    config,
                    ui,
                    Some(manifest.session.name.clone()),
                    step.command.clone(),
                    &no_secrets,
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
                ColabError::config("resume needs a checkpoint - run `colab continue list`")
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
            if let Some(exp_key) = key.strip_prefix("experiments.") {
                return handle_experiment_set(exp_key, value, json);
            }
            let mut cfg =
                config::CocliConfig::load(&path).map_err(|e| ColabError::config(e.to_string()))?;
            match key.as_str() {
                "ui.bell" => cfg.ui.bell = parse_bool(&value)?,
                "ui.animations" => cfg.ui.animations = parse_bool(&value)?,
                "ui.color" => {
                    cfg.ui.color = value.parse()?;
                }
                "ui.compact" => cfg.ui.compact = parse_bool(&value)?,
                "ui.fun" => cfg.ui.fun = parse_bool(&value)?,
                "ui.icons" => cfg.ui.icons = parse_bool(&value)?,
                "ui.neon" => cfg.ui.neon = parse_bool(&value)?,
                "ui.theme" => cfg.ui.theme = value,
                "ui.tui" => {
                    if !matches!(value.as_str(), "auto" | "always" | "never") {
                        return Err(ColabError::config("ui.tui must be auto, always, or never"));
                    }
                    cfg.ui.tui = value;
                }
                "ui.unicode" => cfg.ui.unicode = parse_bool(&value)?,
                "output.json" => cfg.output.json = parse_bool(&value)?,
                "output.quiet" => cfg.output.quiet = parse_bool(&value)?,
                "output.verbose" => cfg.output.verbose = parse_bool(&value)?,
                "output.timestamps" => cfg.output.timestamps = parse_bool(&value)?,
                "skills.enabled" => cfg.skills.enabled = parse_bool(&value)?,
                "support.redact_paths" => cfg.support.redact_paths = parse_bool(&value)?,
                "support.redact_emails" => cfg.support.redact_emails = parse_bool(&value)?,
                "support.redact_tokens" => cfg.support.redact_tokens = parse_bool(&value)?,
                "secrets.provider" => cfg.secrets.provider = value,
                "secrets.allow_env" => cfg.secrets.allow_env = parse_bool(&value)?,
                "secrets.allow_env_file" => cfg.secrets.allow_env_file = parse_bool(&value)?,
                "secrets.redact_names" => cfg.secrets.redact_names = parse_bool(&value)?,
                "secrets.inject_into_notebooks" => {
                    cfg.secrets.inject_into_notebooks = parse_bool(&value)?
                }
                "dev.enabled" => cfg.dev.enabled = parse_bool(&value)?,
                _ => {
                    return Err(ColabError::config(
                        "supported settings keys include ui.theme, ui.color, ui.animations, ui.tui, ui.bell, ui.fun, ui.icons, output.json, skills.enabled, support.redact_tokens, secrets.allow_env, experiments.distribute, experiments.continue, experiments.secrets_bridge, and dev.enabled",
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
    println!(
        "{}",
        heading("Distribute plan", Ui::new(false, false, false))
    );
    let rows = vec![
        vec!["name".to_string(), plan.name.clone()],
        vec!["runtimes".to_string(), plan.requested_runtimes.to_string()],
        vec!["shards".to_string(), plan.shard_count.to_string()],
        vec!["parallel".to_string(), plan.max_parallel_tasks.to_string()],
        vec!["budget".to_string(), plan.budget_limit.to_string()],
        vec!["stop".to_string(), plan.stop_condition.clone()],
        vec!["fast path".to_string(), plan.fast_path.to_string()],
    ];
    print!(
        "{}",
        crate::cocli::ui::table::render_table(
            &["Field", "Value"],
            &rows,
            crate::cocli::ui::width::terminal_width()
        )
    );
    if !findings.is_empty() {
        println!();
        let finding_rows: Vec<Vec<String>> = findings
            .iter()
            .map(|finding| {
                vec![
                    format!("{:?}", finding.level),
                    finding.message.clone(),
                    finding.next_action.clone(),
                ]
            })
            .collect();
        print!(
            "{}",
            crate::cocli::ui::table::render_table(
                &["Level", "Message", "Fix"],
                &finding_rows,
                crate::cocli::ui::width::terminal_width()
            )
        );
    }
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

fn shape_from_args(args: &SessionNewArgs) -> Result<Shape> {
    match (args.shape.as_deref(), args.high_ram) {
        (Some("standard"), false) | (None, false) => Ok(Shape::Standard),
        (Some("high-ram"), _) | (None, true) => Ok(Shape::HighMem),
        (Some("standard"), true) => Err(ColabError::config(
            "choose --shape standard or --high-ram, not both",
        )),
        _ => Err(ColabError::config("shape must be standard or high-ram")),
    }
}

fn session_retries(args: &SessionNewArgs) -> u8 {
    if args.no_retry {
        1
    } else {
        args.retries.clamp(1, 10)
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
        "fs edit is not wired yet; use `fs download`, edit locally, then `fs upload`",
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
            "fs sync currently supports --dry-run planning; use `fs upload` for writes",
        ));
    }
    let plan = local_sync_plan(&args.local, &args.include, &args.exclude, args.delete)?;
    if json {
        return print_value(true, &plan);
    }
    if args.explain {
        println!("fs sync dry-run");
        println!("local: {}", args.local);
        println!("remote: {}", args.remote);
        println!("upload: {} file(s)", plan.upload.len());
        println!("delete remote: {} file(s)", plan.delete_remote.len());
        println!("unchanged: {} file(s)", plan.unchanged);
    } else {
        ui.success("sync dry-run planned");
        println!("upload: {} file(s)", plan.upload.len());
        println!("delete remote: {} file(s)", plan.delete_remote.len());
        println!("unchanged: {} file(s)", plan.unchanged);
    }
    Ok(())
}

fn handle_fs_diff(args: FsDiffArgs, _ui: Ui, json: bool) -> Result<()> {
    let plan = local_sync_plan(&args.local, &args.include, &args.exclude, false)?;
    if json {
        return print_value(true, &plan);
    }
    println!("fs diff");
    println!("upload: {} file(s)", plan.upload.len());
    println!("delete remote: {} file(s)", plan.delete_remote.len());
    println!("unchanged: {} file(s)", plan.unchanged);
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
            "colab status runtime --backend"
        }
        RuntimeCommands::Info { backend: false } => "colab status runtime",
        RuntimeCommands::Gpu => "colab status runtime --gpu",
        RuntimeCommands::Tpu => "colab status runtime --tpu",
        RuntimeCommands::Versions => "colab status runtime --versions",
        RuntimeCommands::Fit { .. } => "colab status runtime --fit MODEL",
    }
}

fn mount_migration_target(cmd: &MountCommands) -> &'static str {
    match cmd {
        MountCommands::Drive { .. } => "colab fs drive mount",
        MountCommands::List { .. } => "colab fs drive status",
    }
}

fn config_migration_target(cmd: &ConfigCommands) -> &'static str {
    match cmd {
        ConfigCommands::Get => "colab settings get",
        ConfigCommands::Set { .. } => "colab settings set KEY VALUE",
        ConfigCommands::Path => "colab settings path",
        ConfigCommands::Open => "colab settings edit",
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

fn print_auth_status(value: serde_json::Value) {
    let signed_in = value
        .get("signed_in")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let account = value
        .get("account")
        .and_then(|v| v.as_str())
        .unwrap_or("not signed in");
    let adc_available = value
        .get("adc_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let adc_path = value
        .get("adc_path")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");
    println!("Auth");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "{:<12} {}",
        "Google",
        if signed_in { account } else { "not signed in" }
    );
    println!(
        "{:<12} {}",
        "ADC",
        if adc_available {
            "available"
        } else {
            "missing"
        }
    );
    println!("{:<12} {}", "ADC path", adc_path);
    if !signed_in {
        println!();
        println!("fix: run colab auth login");
    }
}

fn print_auth_profile_status(value: &serde_json::Value) {
    println!("Auth profile");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for key in ["name", "kind", "account_hint", "storage_backend"] {
        let shown = value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        println!("{:<16} {}", key.replace('_', " "), shown);
    }
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
                AssignOptions {
                    variant,
                    accelerator,
                    name,
                    shape: shape_from(high_ram),
                    keepalive,
                    retries: 1,
                },
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
        ServerCommands::Shell { name } => {
            let no_secrets = SecretBundle::default();
            handle_shell(config, ui, name, &no_secrets).await
        }
        ServerCommands::Info { name } => handle_info(config, ui, name).await,
        ServerCommands::Ps { name, interval } => handle_ps(config, ui, name, interval).await,
        ServerCommands::Run { name, command } => {
            let no_secrets = SecretBundle::default();
            handle_run(config, ui, name, command, &no_secrets).await
        }
    }
}

// `colab server run -- <argv>` — stream remote stdout/stderr, propagate exit code
async fn handle_run(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    command: Vec<String>,
    secrets: &SecretBundle,
) -> Result<()> {
    let manager = make_manager(config)?;
    let servers = manager.list_local()?;
    let server = resolve_server(&servers, name.as_deref())?;
    let server = ensure_fresh_token(&manager, server, &ui).await?;
    let client = manager.client();

    let exit_code =
        runner::run_passthrough_with_secrets(client, &server, &command, secrets).await?;
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

struct AssignOptions {
    variant: Option<Variant>,
    accelerator: Option<String>,
    name: Option<String>,
    shape: Shape,
    keepalive: bool,
    retries: u8,
}

struct AssignRequest {
    label: String,
    variant: Variant,
    accelerator: Option<String>,
    shape: Shape,
    retries: u8,
}

async fn handle_assign(config: &ColabConfig, ui: Ui, options: AssignOptions) -> Result<()> {
    let manager = make_manager(config)?;
    let client = manager.client();

    let ccu = client.get_ccu_info().await.ok();
    let servers = manager.list_local()?;

    let fully_specified = options.variant.is_some() && options.name.is_some();

    if fully_specified || ui.quiet {
        let variant = options.variant.unwrap_or(Variant::Cpu);
        let accelerator = options.accelerator;
        let label = options
            .name
            .unwrap_or_else(|| default_label(variant, accelerator.as_deref()));
        let server = do_assign(
            &manager,
            &ui,
            &ccu,
            AssignRequest {
                label,
                variant,
                accelerator,
                shape: options.shape,
                retries: options.retries,
            },
        )
        .await?;
        if options.keepalive {
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
        if options.keepalive {
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
        .default(if matches!(options.shape, Shape::HighMem) {
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

    let server = do_assign(
        &manager,
        &ui,
        &ccu,
        AssignRequest {
            label,
            variant,
            accelerator,
            shape,
            retries: options.retries,
        },
    )
    .await?;
    if options.keepalive {
        return run_keepalive_loop(config, ui, server).await;
    }
    Ok(())
}

async fn do_assign(
    manager: &ServerManager,
    ui: &Ui,
    ccu: &Option<crate::cocli::session::model::CcuInfo>,
    request: AssignRequest,
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

    let attempts = request.retries.max(1);
    let mut last_error = None;
    for attempt in 1..=attempts {
        let pb = ui.spinner(&format!(
            "assigning runtime · attempt {attempt}/{attempts} · {} {}",
            request.shape.display_name(),
            request
                .accelerator
                .as_deref()
                .unwrap_or_else(|| request.variant.display_name())
        ));
        match manager
            .assign(
                request.label.clone(),
                request.variant,
                request.accelerator.clone(),
                request.shape,
            )
            .await
        {
            Ok(outcome) => {
                Ui::spinner_done(pb, "Assigned");
                println!();
                ui.success("runtime warmed up");
                if outcome.shape_mismatch {
                    ui.warn(&format!(
                        "Requested {} but Colab provisioned {}. Your account tier may not allow {} shape.",
                        outcome.requested_shape,
                        outcome.reported_shape.unwrap_or(Shape::Standard),
                        outcome.requested_shape,
                    ));
                }
                ui.print_server_status(&outcome.server);
                return Ok(outcome.server);
            }
            Err(e) => {
                Ui::spinner_fail(pb, &e.to_string());
                if !assignment_retryable(&e) || attempt == attempts {
                    return Err(e);
                }
                last_error = Some(e);
                tokio::time::sleep(retry_delay(attempt)).await;
                continue;
            }
        }
    }
    Err(last_error.unwrap_or_else(|| ColabError::config("runtime assignment failed")))
}

fn assignment_retryable(e: &ColabError) -> bool {
    matches!(e, ColabError::ApiError { status, .. } if retryable_status(*status))
}

fn retry_delay(attempt: u8) -> std::time::Duration {
    let base = 250u64.saturating_mul(1 << u32::from(attempt.saturating_sub(1)).min(4));
    let jitter = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_millis() % 150))
        .unwrap_or(0);
    std::time::Duration::from_millis(base + jitter)
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

async fn handle_shell(
    config: &ColabConfig,
    ui: Ui,
    name: Option<String>,
    secrets: &SecretBundle,
) -> Result<()> {
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

    runner::run_shell_with_secrets(client, &server, None, Some(refresher), secrets).await
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
    let selected = match name {
        Some("-") => servers
            .iter()
            .max_by_key(|s| s.date_assigned)
            .ok_or_else(|| ColabError::config("no active session - run `colab session list`")),
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
    }?;
    debug::debug1(format!("selected session name={:?}", selected.label));
    debug::debug2(format!(
        "selected session endpoint={} shape={} variant={}",
        selected.endpoint, selected.shape, selected.variant
    ));
    Ok(selected)
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
    debug::debug2(format!(
        "session token remaining={}s refresh_threshold=300s",
        remaining.num_seconds()
    ));
    if remaining.num_seconds() < 5 * 60 {
        debug::debug1(format!(
            "session token refresh start name={:?}",
            server.label
        ));
        let pb = ui.spinner("Refreshing connection token\u{2026}");
        match manager.refresh(server.id).await {
            Ok(updated) => {
                Ui::spinner_done(pb, "Token refreshed");
                debug::debug1("session token refresh ok");
                Ok(updated)
            }
            Err(e) => {
                Ui::spinner_fail(pb, &e.to_string());
                debug::debug1(format!("session token refresh failed error={e}"));
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
    fn repl_multiline_detection_is_conservative() {
        assert!(!python_needs_more_input(&["print(1)".to_string()]));
        assert!(python_needs_more_input(&["def f():".to_string()]));
        assert!(python_needs_more_input(&[
            "def f():".to_string(),
            "    return 1".to_string()
        ]));
        assert!(!python_needs_more_input(&[
            "def f():".to_string(),
            "    return 1".to_string(),
            "".to_string()
        ]));
        assert!(python_needs_more_input(&["print(".to_string()]));
    }

    #[test]
    fn julia_outline_is_basic_and_language_specific() {
        let outline = julia_outline(
            "main.jl",
            "using CSV, DataFrames\nmodule M\nstruct Row\nend\nfunction train(x)\nend\n",
        );
        assert_eq!(outline.kind, "julia-basic");
        assert!(outline.imports.contains(&"CSV".to_string()));
        assert!(outline.classes.contains(&"M".to_string()));
        assert!(outline.functions.contains(&"train".to_string()));
    }

    #[test]
    fn r_outline_is_basic_and_language_specific() {
        let outline = r_outline(
            "main.R",
            "library(dplyr)\ntrain <- function(x) x\nsource('prep.R')\n",
        );
        assert_eq!(outline.kind, "r-basic");
        assert!(outline.imports.contains(&"dplyr".to_string()));
        assert!(outline.functions.contains(&"train".to_string()));
        assert!(
            outline
                .top_level_calls
                .contains(&"source('prep.R')".to_string())
        );
    }

    #[test]
    fn settings_state_navigates_and_batches_edits() {
        let cfg = config::CocliConfig::default();
        let mut state =
            SettingsEditorState::new(PathBuf::from("config.toml"), cfg, SettingsPage::Main);
        state.selected = 1;
        state.activate_selected();
        assert_eq!(state.page, SettingsPage::Ui);
        state.selected = 3;
        state.activate_selected();
        state.selected = 4;
        state.activate_selected();
        assert!(!state.cfg.ui.animations);
        assert!(state.cfg.ui.bell);
        assert!(state.dirty);
        assert!(!state.back_or_exit().unwrap());
        assert_eq!(state.page, SettingsPage::Main);
    }

    #[test]
    fn settings_state_locks_multi_login_until_distribute() {
        let cfg = config::CocliConfig::default();
        let mut state =
            SettingsEditorState::new(PathBuf::from("config.toml"), cfg, SettingsPage::Experiments);
        state.selected = 2;
        state.activate_selected();
        assert!(!state.cfg.experiments.multi_login);
        assert!(
            state
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("locked")
        );
        state.selected = 1;
        state.activate_selected();
        state.selected = 2;
        state.activate_selected();
        assert!(state.cfg.experiments.distribute);
        assert!(state.cfg.experiments.multi_login);
    }

    #[test]
    fn settings_editor_text_is_vertical_and_width_bounded() {
        let cfg = config::CocliConfig::default();
        let ui = Ui::new(false, true, false);
        for width in [60, 80, 100, 140] {
            let mut state = SettingsEditorState::new(
                PathBuf::from("config.toml"),
                cfg.clone(),
                SettingsPage::Main,
            );
            state.selected = 1;
            let text = settings_editor_text(&state, ui, width);
            assert!(text.contains("Settings"));
            assert!(text.contains("> UI"));
            assert!(!text.contains('\r'));
            for line in text.lines() {
                assert!(
                    line.chars().count() <= width,
                    "line exceeded width {width}: {line:?}"
                );
            }
        }
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
            Some("colab fs drive mount")
        );

        let unknown = parse_drive_status("", "/content/drive");
        assert_eq!(unknown.mounted, None);
        assert_eq!(unknown.next_action.as_deref(), Some("colab status check"));
    }

    #[test]
    fn drive_kernel_traceback_gets_friendly_error() {
        let raw = "AttributeError: 'NoneType' object has no attribute 'kernel'";
        let Some(ColabError::Drive(drive)) = classify_drive_error(raw) else {
            panic!("expected drive error");
        };
        assert_eq!(drive.kind, "drive_kernel_context_required");
        assert_eq!(
            drive.message,
            "Drive mount needs a Colab kernel session, not a plain Python process"
        );
        assert_eq!(
            drive.next_action.as_deref(),
            Some("colab session url --open")
        );
        assert!(drive.raw.as_deref().unwrap_or_default().contains("kernel"));
    }

    #[test]
    fn drive_auth_request_gets_browser_approval_error() {
        let raw = "google.colab._message.blocking_request request_auth";
        let Some(ColabError::Drive(drive)) = classify_drive_error(raw) else {
            panic!("expected drive error");
        };
        assert_eq!(drive.kind, "drive_browser_approval_required");
        assert_eq!(drive.message, "Drive needs browser approval");
        assert_eq!(
            drive.next_action.as_deref(),
            Some("open the session once, then run fs drive mount again: colab session url --open")
        );
    }

    #[test]
    fn html_api_body_is_trimmed_for_normal_errors() {
        let raw = "<html><head><title>503 Service Unavailable</title></head><body><h1>Busy</h1></body></html>";
        let trimmed = trim_raw(raw);
        assert!(trimmed.contains("503 Service Unavailable"));
        assert!(!trimmed.contains("<html>"));
        assert!(!trimmed.contains("<body>"));
    }

    #[test]
    fn assignment_503_is_retryable_and_suggests_standard_shape() {
        let url = "https://colab.research.google.com/tun/m/assign?shape=hm";
        let err = ColabError::ApiError {
            status: 503,
            url: url.to_string(),
            body: Some("<html>busy</html>".to_string()),
        };
        assert!(assignment_retryable(&err));
        assert_eq!(
            api_fix(503, url),
            Some("run again with Standard RAM: colab session new --shape standard")
        );
    }

    #[test]
    fn drive_endpoint_error_is_structured_and_retryable() {
        let server = StoredServer {
            id: uuid::Uuid::nil(),
            label: "Colab CPU".to_string(),
            variant: Variant::Cpu,
            accelerator: None,
            shape: Shape::Standard,
            endpoint: "stale-endpoint".to_string(),
            proxy_url: "https://example.invalid".to_string(),
            proxy_token: "redacted".to_string(),
            token_expires_at: chrono::Utc::now(),
            date_assigned: chrono::Utc::now(),
            selected_kernel_id: None,
            selected_kernel_name: None,
            kernel_language: None,
            kernel_language_version: None,
            kernel_cache_stale: false,
        };
        let err = drive_endpoint_error(
            "check_jupyter_sessions",
            &server,
            "unknown_network",
            true,
            Some("error sending request for url".to_string()),
        );
        let ColabError::Drive(drive) = err else {
            panic!("expected drive error");
        };
        assert_eq!(drive.kind, "unknown_network");
        assert_eq!(drive.stage.as_deref(), Some("check_jupyter_sessions"));
        assert!(drive.retryable);
        assert!(drive.message.contains("Runtime endpoint is not reachable"));
        assert!(drive.message.contains("Colab CPU"));
        assert!(
            drive
                .fixes
                .iter()
                .any(|fix| fix.contains("session list --refresh"))
        );
        assert!(drive.raw.as_deref().unwrap_or_default().contains("request"));
    }
}
