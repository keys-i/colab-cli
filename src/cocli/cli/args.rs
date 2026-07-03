use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "colab-cli",
    about = "Google Colab from the terminal",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(long, short, global = true, env = "COLAB_QUIET")]
    pub quiet: bool,

    #[arg(long)]
    pub json: bool,

    #[arg(long, global = true)]
    pub verbose: bool,

    #[arg(long, global = true, default_value = "auto", value_parser = ["auto", "always", "never"])]
    pub color: String,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub bell: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// create, list, stop, and open Colab sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
    /// run code and prepare the runtime
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },
    /// move files and manage Drive
    Fs {
        #[command(subcommand)]
        command: FsCommands,
    },
    /// show runtime, auth, session, and health state
    Status {
        #[command(subcommand)]
        command: Option<StatusCommands>,
    },
    /// save and resume checkpoint plans
    #[command(name = "continue")]
    Continue {
        #[command(subcommand)]
        command: ContinueCommands,
    },
    /// run tiny TOML workflows
    Slurp {
        #[command(subcommand)]
        command: SlurpCommands,
    },
    /// plan approved multi-runtime work
    Fleet {
        #[command(subcommand)]
        command: FleetCommands,
    },
    /// edit config and inspect skills
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
    },
    /// release metadata and maintainer helpers
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
    },
    /// Authentication
    #[command(hide = true)]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Generate shell completions
    #[command(hide = true)]
    Completions { shell: clap_complete::Shell },

    /// Write a redacted diagnostic bundle
    #[command(name = "bug-report", hide = true)]
    BugReport {
        #[arg(long)]
        show_private: bool,
    },
    /// Compatibility: `exec` moved to `run`.
    #[command(hide = true)]
    Exec {
        #[command(subcommand)]
        command: ExecCommands,
    },
    /// Compatibility: `env` moved to `run`.
    #[command(hide = true)]
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    /// Compatibility: `mount` moved under `fs drive`.
    #[command(hide = true)]
    Mount {
        #[command(subcommand)]
        command: MountCommands,
    },
    /// Compatibility: `runtime` moved to `status runtime`.
    #[command(hide = true)]
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommands,
    },
    /// Compatibility: `tools` moved to `settings skills`.
    #[command(hide = true)]
    Tools {
        #[command(subcommand)]
        command: ToolsCommands,
    },
    /// Compatibility: `config` moved to `settings`.
    #[command(hide = true)]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Compatibility: `doctor` moved to `status check`.
    #[command(hide = true)]
    Doctor {
        #[arg(long)]
        vibe: bool,
        #[command(subcommand)]
        command: Option<DoctorCommands>,
    },
    /// Compatibility: old agent surfaces.
    #[command(hide = true)]
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Compatibility: old Rust `server` group.
    #[command(hide = true)]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Compatibility: old Rust `file` group.
    #[command(hide = true)]
    File {
        #[command(subcommand)]
        command: FileCommands,
    },
    /// Compatibility: `colab new`.
    #[command(name = "new", hide = true)]
    CompatNew(SessionNewArgs),
    /// Compatibility: `colab sessions`.
    #[command(name = "sessions", hide = true)]
    CompatSessions,
    /// Compatibility: `colab stop`.
    #[command(name = "stop", hide = true)]
    CompatStop(SessionNameArg),
    /// Compatibility: `colab upload`.
    #[command(name = "upload", hide = true)]
    CompatUpload(CompatTransferArgs),
    /// Compatibility: `colab download`.
    #[command(name = "download", hide = true)]
    CompatDownload(CompatTransferArgs),
}

#[derive(clap::Args)]
pub struct SessionNameArg {
    #[arg(long, short = 's', alias = "name")]
    pub session: Option<String>,
}

#[derive(clap::Args)]
pub struct CompatTransferArgs {
    #[arg(long, short = 's', alias = "name")]
    pub session: Option<String>,
    pub src: String,
    pub dest: String,
}

#[derive(clap::Args)]
pub struct SessionNewArgs {
    #[arg(long, short = 's')]
    pub name: Option<String>,
    #[arg(long)]
    pub gpu: Option<String>,
    #[arg(long)]
    pub tpu: Option<String>,
    #[arg(long = "high-ram")]
    pub high_ram: bool,
    #[arg(long, short = 'k')]
    pub keepalive: bool,
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Sign in to Google (opens browser)
    Login,
    /// Sign out and clear stored credentials
    Logout,
    Add(AuthProfileArgs),
    List {
        #[arg(long)]
        show_private: bool,
    },
    Status {
        #[arg(long)]
        name: String,
        #[arg(long)]
        show_private: bool,
    },
    Use {
        #[arg(long)]
        name: String,
        #[arg(long)]
        allow_fallback_account: bool,
    },
    Remove {
        #[arg(long)]
        name: String,
    },
    Doctor,
    ExportRedacted {
        #[arg(long)]
        show_private: bool,
    },
    Limits {
        #[arg(long)]
        name: String,
    },
}

#[derive(clap::Args)]
pub struct AuthProfileArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long, default_value = "unknown")]
    pub kind: String,
    #[arg(long)]
    pub account_hint: Option<String>,
    #[arg(long)]
    pub session_only: bool,
}

#[derive(Subcommand)]
pub enum SessionCommands {
    /// Assign a new Colab session
    New(SessionNewArgs),
    /// List assigned sessions
    #[command(alias = "ls")]
    List,
    /// Compatibility: moved to `status session`.
    #[command(hide = true)]
    Status(SessionNameArg),
    /// Stop a session
    Stop(SessionNameArg),
    /// Print a session URL
    Url {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long)]
        open: bool,
    },
    /// Show the last assigned session
    Last,
}

#[derive(Subcommand)]
pub enum RunCommands {
    /// Run a Python script path on the runtime
    #[command(alias = "run")]
    Script {
        script: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run Python code
    Py {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        code: String,
    },
    /// Execute a notebook with nbconvert on the runtime
    #[command(alias = "nb")]
    Notebook {
        notebook: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        out: Option<String>,
    },
    /// Start a Python REPL
    Repl {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Start a remote shell
    Shell {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Install packages or a requirements file
    Install {
        packages: Vec<String>,
        #[arg(short = 'r', long = "requirements")]
        requirements: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Freeze installed packages
    Freeze {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Restore packages from requirements.txt
    Restore {
        requirements: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Rerun the last local command after confirmation
    Last {
        #[arg(long)]
        confirm: bool,
    },
    /// Show recent local run commands
    History,
}

#[derive(Subcommand)]
pub enum ExecCommands {
    /// Run a Python script path on the runtime
    Run {
        script: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run Python code
    Py {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        code: String,
    },
    /// Execute a notebook with nbconvert on the runtime
    Nb {
        notebook: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        out: Option<String>,
    },
    /// Start a Python REPL
    Repl {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Start a remote shell
    Shell {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Rerun the last local command after confirmation
    Last {
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand)]
pub enum FsCommands {
    /// List remote files
    Ls {
        path: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Push a local file
    Push {
        src: String,
        dest: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Pull a remote file or directory
    Pull {
        src: String,
        dest: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Remove remote files
    Rm {
        path: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        recursive: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Edit a remote file
    Edit {
        path: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Plan local-to-remote sync
    Sync(FsSyncArgs),
    /// Diff local and remote manifests
    Diff(FsDiffArgs),
    /// Show local changes that sync would upload
    Changed(FsDiffArgs),
    /// Manage Google Drive inside the runtime filesystem
    Drive {
        #[command(subcommand)]
        command: FsDriveCommands,
    },
}

#[derive(Subcommand)]
pub enum FsDriveCommands {
    /// Mount Google Drive
    Mount {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long, default_value = "/content/drive")]
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Show Drive mount state
    Status {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Unmount Google Drive
    Unmount {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the expected Drive path
    Path {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(clap::Args)]
pub struct FsSyncArgs {
    pub local: String,
    pub remote: String,
    #[arg(long, short = 's')]
    pub session: Option<String>,
    #[arg(long)]
    pub include: Vec<String>,
    #[arg(long)]
    pub exclude: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub explain: bool,
    #[arg(long)]
    pub delete: bool,
    #[arg(long)]
    pub watch: bool,
}

#[derive(clap::Args)]
pub struct FsDiffArgs {
    pub local: String,
    pub remote: String,
    #[arg(long, short = 's')]
    pub session: Option<String>,
    #[arg(long)]
    pub include: Vec<String>,
    #[arg(long)]
    pub exclude: Vec<String>,
}

#[derive(Subcommand)]
pub enum MountCommands {
    /// Mount Google Drive
    Drive {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long, default_value = "/content/drive")]
        path: String,
    },
    /// List known mounts
    List {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    /// Install packages
    Install {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Freeze installed packages
    Freeze {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Restore packages from requirements.txt
    Restore {
        requirements: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RuntimeCommands {
    Info {
        #[arg(long)]
        backend: bool,
    },
    Gpu,
    Tpu,
    Versions,
    Fit {
        #[arg(long)]
        model: String,
    },
    #[command(name = "backend-info", hide = true)]
    BackendInfo,
}

#[derive(Subcommand)]
pub enum StatusCommands {
    /// Show session state
    Session {
        #[arg(long, alias = "name")]
        name: Option<String>,
    },
    /// Show runtime metadata
    Runtime {
        #[arg(long)]
        backend: bool,
        #[arg(long)]
        gpu: bool,
        #[arg(long)]
        tpu: bool,
        #[arg(long)]
        versions: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        fit: Option<String>,
    },
    /// Show auth state
    Auth,
    /// Show file sync state
    Fs,
    /// Show Drive state
    Drive,
    /// Show Slurp config state
    Slurp {
        #[arg(long, default_value = "slurp.toml")]
        config: String,
    },
    /// Show fleet planning state
    Fleet {
        #[arg(long, default_value = "slurp.toml")]
        config: String,
    },
    /// Fast local health check
    Quick,
    /// Run local health checks
    Check,
    /// Show runtime setup hints
    Run,
    /// Show config/cache paths
    Paths,
}

#[derive(Subcommand)]
pub enum ToolsCommands {
    List {
        #[arg(long)]
        json: bool,
    },
    Run {
        tool_name: String,
        #[arg(long = "json", default_value = "{}")]
        input_json: String,
        #[arg(long)]
        yes: bool,
    },
    Inspect {
        tool_name: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum SettingsCommands {
    Get,
    Set {
        key: String,
        value: String,
    },
    #[command(alias = "locate")]
    Path,
    Edit,
    Reset {
        #[arg(long)]
        yes: bool,
    },
    Skills {
        #[command(subcommand)]
        command: SkillCommands,
    },
}

#[derive(Subcommand)]
pub enum SkillCommands {
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        risk: Option<String>,
        #[arg(long)]
        needs_session: bool,
    },
    Inspect {
        name: String,
        #[arg(long)]
        json: bool,
    },
    Run {
        name: String,
        #[arg(long = "json", default_value = "{}")]
        input_json: String,
        #[arg(long)]
        yes: bool,
    },
    Enable {
        name: String,
    },
    Disable {
        name: String,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    Tools,
    Plan {
        goal: String,
        #[arg(long)]
        out: Option<String>,
    },
    Run {
        plan: String,
        #[arg(long)]
        confirm: bool,
    },
    AuditPlan {
        plan: String,
    },
    Slurp {
        goal: String,
        #[arg(long)]
        out: Option<String>,
    },
    AuditSlurp {
        config: String,
    },
    ExplainSlurp {
        config: String,
    },
}

#[derive(Subcommand)]
pub enum FleetCommands {
    Plan(FleetConfigArgs),
    Start(FleetConfigArgs),
    Exec(FleetConfigArgs),
    #[command(hide = true)]
    Doctor,
}

#[derive(clap::Args, Clone)]
pub struct FleetConfigArgs {
    #[arg(long, default_value = "slurp.toml")]
    pub config: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub cost: bool,
    #[arg(long)]
    pub allow_fallback_account: bool,
}

#[derive(Subcommand)]
pub enum SlurpCommands {
    Init {
        #[arg(long, default_value = "slurp.toml")]
        out: String,
    },
    Check(FleetConfigArgs),
    Plan(FleetConfigArgs),
    Run(FleetConfigArgs),
    Resume(FleetConfigArgs),
    Explain(FleetConfigArgs),
    #[command(hide = true)]
    Doctor(FleetConfigArgs),
    #[command(hide = true)]
    Schema,
}

#[derive(Subcommand)]
pub enum ReleaseCommands {
    Name {
        version: String,
    },
    Notes {
        version: String,
        commits: Vec<String>,
    },
    Bump {
        commits: Vec<String>,
        #[arg(long)]
        pre_1: bool,
    },
}

#[derive(Subcommand)]
pub enum ContinueCommands {
    Save {
        #[arg(long, short = 's')]
        session: String,
        #[arg(long)]
        name: String,
        #[arg(long = "artifact")]
        artifacts: Vec<String>,
    },
    Resume {
        name: String,
        #[arg(long)]
        new_runtime: bool,
        #[arg(long)]
        gpu: Option<String>,
        #[arg(long)]
        replay_all: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Last,
    Export {
        name: String,
        #[arg(long)]
        out: String,
    },
    Import {
        bundle: String,
    },
    Inspect {
        name: String,
    },
    Clean {
        #[arg(long)]
        older_than: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    Get,
    Set {
        key: String,
        value: String,
    },
    #[command(alias = "locate")]
    Path,
    Open,
}

#[derive(Subcommand)]
pub enum DoctorCommands {
    Quick,
    Auth,
    Mounts,
    Env,
    Paths,
    Perf,
    Compliance,
    #[command(hide = true)]
    Ferret,
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Assign a new Colab server (interactive if no flags given)
    Assign {
        #[arg(long, value_parser = parse_variant)]
        variant: Option<crate::cocli::session::model::Variant>,

        #[arg(long, short)]
        accelerator: Option<String>,

        #[arg(long)]
        name: Option<String>,

        /// Request a high-memory machine shape
        #[arg(long = "high-ram")]
        high_ram: bool,

        /// Keep the server alive indefinitely (pings + auto-refresh tokens)
        #[arg(long, short = 'k')]
        keepalive: bool,
    },
    /// Reconfigure an existing server (variant / accelerator / shape)
    Reconfigure {
        #[arg(long)]
        name: Option<String>,

        #[arg(long, value_parser = parse_variant)]
        variant: Option<crate::cocli::session::model::Variant>,

        #[arg(long, short)]
        accelerator: Option<String>,

        #[arg(long = "high-ram")]
        high_ram: bool,

        /// Keep the server alive indefinitely after reconfigure (pings + token refresh)
        #[arg(long, short = 'k')]
        keepalive: bool,
    },
    /// List assigned servers, or available accelerators with `--available`
    Ls {
        /// Show available accelerator choices with CCU/hr rates instead
        #[arg(long, short = 'a')]
        available: bool,
    },
    /// Remove an assigned server
    Rm {
        #[arg(long)]
        name: Option<String>,
    },
    /// Open an interactive shell on a server
    Shell {
        #[arg(long)]
        name: Option<String>,
    },
    /// Show server and account info
    Info {
        #[arg(long)]
        name: Option<String>,
    },
    /// Realtime system stats (CPU / RAM / disk / GPU) for a server
    Ps {
        #[arg(long)]
        name: Option<String>,
        /// Refresh interval in milliseconds
        #[arg(long, default_value_t = 1000)]
        interval: u64,
    },
    /// Run an arbitrary command on the assigned server (passthrough)
    Run {
        #[arg(long)]
        name: Option<String>,

        /// Command and arguments to execute on the remote runtime.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum FileCommands {
    /// Upload a local file to the runtime
    Upload {
        #[arg(long)]
        name: Option<String>,
        src: String,
        dest: Option<String>,
    },
    /// Download a file or directory from the runtime to the local machine
    Download {
        #[arg(long)]
        name: Option<String>,
        src: String,
        dest: Option<String>,
    },
    /// List files on the runtime (passes args through to remote `ls`)
    Ls {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `ls`. Defaults to `-lah /content`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Copy files on the runtime (passes args through to remote `cp`)
    Cp {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `cp`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
    /// Remove files on the runtime (passes args through to remote `rm`)
    Rm {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `rm`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
}

fn parse_variant(s: &str) -> std::result::Result<crate::cocli::session::model::Variant, String> {
    match s.to_ascii_lowercase().as_str() {
        "cpu" | "default" => Ok(crate::cocli::session::model::Variant::Cpu),
        "gpu" => Ok(crate::cocli::session::model::Variant::Gpu),
        "tpu" => Ok(crate::cocli::session::model::Variant::Tpu),
        other => Err(format!(
            "unknown variant '{other}' - expected cpu, gpu, or tpu"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cocli::session::model::Variant;
    use clap::CommandFactory;

    #[test]
    fn cli_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_variant_accepts_canonical_forms() {
        assert_eq!(parse_variant("cpu").unwrap(), Variant::Cpu);
        assert_eq!(parse_variant("CPU").unwrap(), Variant::Cpu);
        assert_eq!(parse_variant("default").unwrap(), Variant::Cpu);
        assert_eq!(parse_variant("gpu").unwrap(), Variant::Gpu);
        assert_eq!(parse_variant("GPU").unwrap(), Variant::Gpu);
        assert_eq!(parse_variant("tpu").unwrap(), Variant::Tpu);
        assert_eq!(parse_variant("TPU").unwrap(), Variant::Tpu);
    }

    #[test]
    fn new_command_space_parses() {
        let cli = Cli::try_parse_from([
            "colab-cli",
            "session",
            "new",
            "--name",
            "trainer",
            "--gpu",
            "A100",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Session {
                command: SessionCommands::New(_)
            }
        ));
    }

    #[test]
    fn run_script_parses_forwarded_args() {
        let cli = Cli::try_parse_from([
            "colab-cli",
            "run",
            "script",
            "train.py",
            "--session",
            "trainer",
            "--",
            "--epochs",
            "3",
        ])
        .unwrap();
        let Commands::Run {
            command:
                RunCommands::Script {
                    script,
                    session,
                    args,
                },
        } = cli.command
        else {
            panic!("expected run script");
        };
        assert_eq!(script, "train.py");
        assert_eq!(session.as_deref(), Some("trainer"));
        assert_eq!(args, ["--epochs", "3"]);
    }

    #[test]
    fn fs_pull_and_compat_download_parse() {
        assert!(Cli::try_parse_from(["colab-cli", "fs", "pull", "/content/out", "./out"]).is_ok());
        assert!(Cli::try_parse_from(["colab-cli", "download", "/content/out", "./out"]).is_ok());
    }

    #[test]
    fn new_followup_spaces_parse() {
        assert!(Cli::try_parse_from(["colab-cli", "run", "install", "torch"]).is_ok());
        assert!(Cli::try_parse_from(["colab-cli", "status", "runtime", "--gpu"]).is_ok());
        assert!(Cli::try_parse_from(["colab-cli", "fs", "drive", "mount"]).is_ok());
        assert!(
            Cli::try_parse_from(["colab-cli", "settings", "skills", "inspect", "session.new"])
                .is_ok()
        );
        assert!(
            Cli::try_parse_from(["colab-cli", "fleet", "plan", "--config", "slurp.toml"]).is_ok()
        );
        assert!(Cli::try_parse_from(["colab-cli", "slurp", "explain"]).is_ok());
        assert!(Cli::try_parse_from(["colab-cli", "release", "name", "v0.4.2"]).is_ok());
    }

    #[test]
    fn help_subcommand_is_disabled() {
        let Err(err) = Cli::try_parse_from(["colab-cli", "help"]) else {
            panic!("`colab-cli help` should not parse");
        };
        assert!(matches!(
            err.kind(),
            clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument
        ));
    }
}
