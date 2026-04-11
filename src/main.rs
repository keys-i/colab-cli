use clap::{CommandFactory, Parser};

use colab_cli::auth;
use colab_cli::cli::{AuthCommands, Cli, Commands, FileCommands, ServerCommands};
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
    let ui = Ui::new(cli.quiet);

    if let Err(e) = run(cli, ui).await {
        ui.error(&e.to_string());

        match &e {
            ColabError::NotAuthenticated => {
                eprintln!("  Run `colab auth login` to sign in.");
            }
            ColabError::TooManyAssignments => {
                eprintln!("  Run `colab server rm` to remove one.");
            }
            _ => {}
        }

        std::process::exit(1);
    }
}

async fn run(cli: Cli, ui: Ui) -> Result<()> {
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        clap_complete::generate(*shell, &mut cmd, "colab", &mut std::io::stdout());
        return Ok(());
    }

    let config = ColabConfig::load(cli.quiet)?;

    match cli.command {
        Commands::Auth { command } => match command {
            AuthCommands::Login => handle_login(&config, ui).await,
            AuthCommands::Logout => {
                auth::logout()?;
                ui.success("Signed out. Credentials cleared.");
                Ok(())
            }
        },
        Commands::Server { command } => handle_server(command, &config, ui).await,
        Commands::File { command } => handle_file(command, &config, ui).await,
        Commands::Completions { .. } => unreachable!(),
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

    let auto_connect = if !servers.is_empty() {
        let latest = latest_server(&servers).unwrap();
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
        let server = latest_server(&servers).unwrap().clone();
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
            ui.success(&format!("Assigned server: {}", outcome.server.label));
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
        pb.set_style(
            indicatif::ProgressStyle::with_template(
                "{spinner:.cyan} Uploading [{bar:30}] {bytes}/{total_bytes} ({eta})",
            )
            .unwrap()
            .progress_chars("\u{2588}\u{2593}\u{2591}"),
        );
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
