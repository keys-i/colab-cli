use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "colab",
    about = "Google Colab from the terminal",
    version,
    disable_help_subcommand = true,
    override_usage = "colab [OPTIONS] <COMMAND>",
    help_template = "Google Colab from the terminal\n\nUsage: colab [OPTIONS] <COMMAND>\n\nCommands:\n  session      Manage Colab sessions\n  run          Run code on Colab\n  fs           Files, sync, and Drive\n  status       Session and runtime status\n  auth         Sign in and inspect credentials\n  log          View and export history\n  settings     Config, UI, support, and experiments\n  ai           Agent-facing tools\n  update       Check or install updates\n  version      Show version\n  pay          Open Colab billing / compute units page\n  completions  Generate shell completions\n\nOptions:\n  -q, --quiet\n      --json\n  -v, --verbose\n      --no-color\n      --bell\n  -h, --help\n  -V, --version\n"
)]
pub struct Cli {
    #[arg(long, short, global = true, env = "COLAB_QUIET")]
    pub quiet: bool,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[arg(long, global = true, default_value = "auto", value_name = "auto|always|never", value_parser = ["auto", "always", "never"], hide = true)]
    pub color: String,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub bell: bool,

    #[arg(long, global = true, hide = true)]
    pub no_interactive: bool,

    #[arg(long, global = true, hide = true)]
    pub plain: bool,

    #[arg(long, global = true, hide = true)]
    pub tui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage Colab sessions
    #[command(display_order = 10)]
    Session {
        #[command(subcommand)]
        command: Option<SessionCommands>,
    },
    /// Run code on Colab
    #[command(display_order = 20)]
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },
    /// Files, sync, and Drive
    #[command(display_order = 30)]
    Fs {
        #[command(subcommand)]
        command: FsCommands,
    },
    /// Session and runtime status
    #[command(display_order = 40)]
    Status {
        #[command(subcommand)]
        command: Option<StatusCommands>,
    },
    /// View and export history
    #[command(display_order = 45)]
    Log {
        #[command(subcommand)]
        command: Option<LogCommands>,
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long, default_value_t = 50)]
        tail: usize,
        #[arg(long, default_value = "text", value_parser = ["text", "md", "ipynb", "jsonl"])]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
    /// Checkpoint and resume work
    #[command(name = "continue")]
    #[command(display_order = 50, hide = true)]
    Continue {
        #[command(subcommand)]
        command: ContinueCommands,
    },
    /// Experimental workflow distribution
    #[command(display_order = 55, hide = true)]
    Distribute {
        #[command(subcommand)]
        command: Option<DistributeCommands>,
    },
    /// Tiny TOML workflows
    #[command(display_order = 60, hide = true)]
    Slurp {
        #[command(subcommand)]
        command: SlurpCommands,
    },
    /// Compliant runtime planning
    #[command(display_order = 70, hide = true)]
    Fleet {
        #[command(subcommand)]
        command: FleetCommands,
    },
    /// Agent-facing tools
    #[command(display_order = 75)]
    Ai {
        #[command(subcommand)]
        command: Option<AiCommands>,
    },
    /// Sign in and inspect credentials
    #[command(display_order = 80)]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Config, UI, support, and experiments
    #[command(display_order = 90)]
    Settings {
        #[command(subcommand)]
        command: Option<SettingsCommands>,
    },
    /// Check or install updates
    #[command(display_order = 92)]
    Update {
        #[arg(long)]
        install: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Show version
    #[command(display_order = 94)]
    Version,
    /// Open Colab billing / compute units page
    #[command(display_order = 96)]
    Pay {
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate shell completions
    #[command(display_order = 100)]
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
    #[arg(long, value_parser = ["standard", "high-ram"])]
    pub shape: Option<String>,
    #[arg(long, default_value_t = 3)]
    pub retries: u8,
    #[arg(long)]
    pub no_retry: bool,
    #[arg(long, short = 'k')]
    pub keepalive: bool,
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Sign in to Google (opens browser)
    Login {
        #[arg(long, default_value = "oauth2", value_parser = ["oauth2", "adc"])]
        method: String,
    },
    /// Sign out and clear stored credentials
    Logout { profile: Option<String> },
    #[command(hide = true)]
    Add(AuthProfileArgs),
    List {
        #[arg(long)]
        show_private: bool,
    },
    Status {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        show_private: bool,
    },
    Use {
        #[arg(long)]
        name: String,
        #[arg(long)]
        allow_fallback_account: bool,
    },
    #[command(hide = true)]
    Remove {
        #[arg(long)]
        name: String,
    },
    #[command(hide = true)]
    Doctor,
    ExportRedacted {
        #[arg(long)]
        show_private: bool,
    },
    #[command(hide = true)]
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
    /// Show session status
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
    /// Refresh known sessions and endpoints
    Refresh,
    /// Check whether the selected session looks stale
    Repair(SessionNameArg),
    /// Try to reconnect a local session name to an active runtime
    Reconnect(SessionNameArg),
    /// Show captured session logs where available
    Logs(SessionLogsArgs),
    /// Kernel controls
    Kernel {
        #[command(subcommand)]
        command: SessionKernelCommands,
    },
}

#[derive(clap::Args)]
pub struct SessionLogsArgs {
    #[arg(long, short = 's', alias = "name")]
    pub session: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub tail: usize,
    #[arg(long, default_value = "text", value_parser = ["text", "md", "ipynb", "jsonl"])]
    pub format: String,
    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Subcommand)]
pub enum LogCommands {
    List {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
    },
    Show {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long, default_value_t = 50)]
        tail: usize,
    },
    Export {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long, default_value = "md", value_parser = ["text", "md", "ipynb", "jsonl"])]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
    Tail {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long, default_value_t = 50)]
        lines: usize,
    },
}

#[derive(Subcommand)]
pub enum SessionKernelCommands {
    List(KernelSessionArg),
    Current(KernelSessionArg),
    Select {
        kernel: Option<String>,
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
    },
    Specs(KernelSessionArg),
    Start {
        #[arg(long)]
        spec: String,
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
    },
    Status(KernelSessionArg),
    Interrupt(KernelActionArgs),
    Restart {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long, default_value_t = 60)]
        timeout: u64,
    },
    Shutdown {
        #[arg(long, short = 's', alias = "name")]
        session: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    Refresh(KernelSessionArg),
}

#[derive(clap::Args)]
pub struct KernelSessionArg {
    #[arg(long, short = 's', alias = "name")]
    pub session: Option<String>,
}

#[derive(clap::Args)]
pub struct KernelActionArgs {
    #[arg(long, short = 's', alias = "name")]
    pub session: Option<String>,
    #[arg(long)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum RunCommands {
    /// Run code in the active kernel
    Code {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        code: String,
    },
    /// Run a Python script path on the runtime
    #[command(alias = "run")]
    Script {
        script: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        ast: bool,
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
        #[arg(long)]
        ast: bool,
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
    /// Manage Python packages on the runtime
    Pip {
        #[command(subcommand)]
        command: PipCommands,
    },
    /// Package commands for the active kernel
    Pkg {
        #[command(subcommand)]
        command: PkgCommands,
    },
    /// Julia tools
    #[command(hide = true)]
    Julia {
        #[command(subcommand)]
        command: JuliaCommands,
    },
    /// R tools
    #[command(hide = true)]
    R {
        #[command(subcommand)]
        command: RCommands,
    },
    /// Show a local code outline before execution
    Ast {
        file: String,
        #[arg(long)]
        json: bool,
    },
    /// Watch a local script path before running it
    Watch {
        script: String,
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        ast: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Compatibility: moved to `run pip install`.
    #[command(hide = true)]
    Install {
        packages: Vec<String>,
        #[arg(short = 'r', long = "requirements")]
        requirements: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Compatibility: moved to `run pip freeze`.
    #[command(hide = true)]
    Freeze {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Compatibility: moved to `run pip restore`.
    #[command(hide = true)]
    Restore {
        requirements: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Rerun the last local command after confirmation
    #[command(hide = true)]
    Last {
        #[arg(long)]
        confirm: bool,
    },
    /// Show recent local run commands
    #[command(hide = true)]
    History,
}

#[derive(Subcommand)]
pub enum PkgCommands {
    Add {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Remove {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    List {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Status {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Update {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Restore {
        file: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Check {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum JuliaCommands {
    Pkg {
        #[command(subcommand)]
        command: JuliaPkgCommands,
    },
}

#[derive(Subcommand)]
pub enum JuliaPkgCommands {
    Add {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Status {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Instantiate {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Precompile {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Update {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Test {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Rm {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RCommands {
    Pkg {
        #[command(subcommand)]
        command: RPkgCommands,
    },
    Renv {
        #[command(subcommand)]
        command: RenvCommands,
    },
    SessionInfo {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RPkgCommands {
    Install {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    List {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Update {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Remove {
        packages: Vec<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum RenvCommands {
    Restore {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Snapshot {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PipCommands {
    Install {
        packages: Vec<String>,
        #[arg(short = 'r', long = "requirements")]
        requirements: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Freeze {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Restore {
        requirements: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Check {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    List {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Tree {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    Cache {
        #[arg(long, short = 's')]
        session: Option<String>,
    },
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
    /// Upload a local file
    Upload {
        src: String,
        dest: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Download a remote file or directory
    Download {
        src: String,
        dest: Option<String>,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Compatibility: moved to `fs upload`.
    #[command(hide = true)]
    Push {
        src: String,
        dest: String,
        #[arg(long, short = 's')]
        session: Option<String>,
    },
    /// Compatibility: moved to `fs download`.
    #[command(hide = true)]
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
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        #[arg(long, default_value_t = 10)]
        preflight_timeout: u64,
        #[arg(long, default_value_t = 2)]
        retries: u8,
        #[arg(long)]
        no_retry: bool,
        #[arg(long)]
        open: bool,
    },
    /// Show Drive mount state
    Status {
        #[arg(long, short = 's')]
        session: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// List files under the Drive mount path
    List {
        #[arg(long, short = 's')]
        session: Option<String>,
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
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        #[arg(long, default_value_t = 10)]
        preflight_timeout: u64,
        #[arg(long, default_value_t = 2)]
        retries: u8,
        #[arg(long)]
        no_retry: bool,
        #[arg(long)]
        open: bool,
        #[arg(long)]
        dry_run: bool,
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
    /// Show kernel state
    Kernel {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long, alias = "name")]
        session: Option<String>,
    },
    /// Compatibility: recipe status moved to `distribute status`.
    #[command(hide = true)]
    Slurp {
        #[arg(long, default_value = "slurp.toml")]
        config: String,
    },
    /// Compatibility: pool status moved to `distribute status`.
    #[command(hide = true)]
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
    /// Show build and config version information
    Version,
}

#[derive(Subcommand)]
pub enum ToolsCommands {
    List {
        #[arg(long)]
        json: bool,
    },
    Run {
        tool_name: String,
        #[arg(long = "json-input", default_value = "{}")]
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
    Get {
        key: Option<String>,
    },
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
    #[command(hide = true)]
    Skills {
        #[command(subcommand)]
        command: SkillCommands,
    },
    Ui {
        #[command(subcommand)]
        command: Option<SettingsUiCommands>,
    },
    Experiments {
        #[command(subcommand)]
        command: Option<SettingsExperimentsCommands>,
    },
    Support {
        #[command(subcommand)]
        command: SupportCommands,
    },
    About,
    #[command(hide = true)]
    Update {
        #[command(subcommand)]
        command: SettingsUpdateCommands,
    },
    #[command(hide = true)]
    Billing {
        #[command(subcommand)]
        command: SettingsBillingCommands,
    },
    #[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
    #[command(hide = true)]
    Dev {
        #[command(subcommand)]
        command: DevCommands,
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
        scope: Option<String>,
        #[arg(long)]
        risk: Option<String>,
        #[arg(long)]
        needs_session: bool,
        #[arg(long)]
        enabled: bool,
        #[arg(long)]
        disabled: bool,
    },
    Inspect {
        name: String,
        #[arg(long)]
        json: bool,
    },
    Run {
        name: String,
        #[arg(long = "json-input", default_value = "{}")]
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
    Mcp {
        #[arg(long)]
        stdio: bool,
    },
}

#[derive(Subcommand)]
pub enum SettingsUiCommands {
    Get { key: Option<String> },
    Set { key: String, value: String },
    Reset,
    Preview,
}

#[derive(Subcommand)]
pub enum SettingsExperimentsCommands {
    Get { key: Option<String> },
    Set { key: String, value: String },
    Reset,
}

#[derive(Subcommand)]
pub enum SupportCommands {
    BugReport {
        #[arg(long)]
        show_private: bool,
    },
    Redact {
        text: Option<String>,
    },
    Bundle,
}

#[derive(Subcommand)]
pub enum SettingsUpdateCommands {
    Check,
    Install {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum SettingsBillingCommands {
    Open {
        #[arg(long)]
        dry_run: bool,
    },
    Status,
}

#[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
#[derive(Subcommand)]
pub enum DevCommands {
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
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
pub enum AiCommands {
    Tools {
        #[command(subcommand)]
        command: Option<AiToolsCommands>,
    },
    Mcp {
        #[command(subcommand)]
        command: Option<AiMcpCommands>,
    },
    Plan {
        goal: String,
        #[arg(long)]
        out: Option<String>,
    },
    Audit {
        plan_file: String,
    },
    Explain {
        plan_file: String,
    },
    Run {
        plan_file: String,
        #[arg(long)]
        confirm: bool,
    },
    #[command(hide = true)]
    Ast {
        first: String,
        second: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Code {
        #[command(subcommand)]
        command: AiCodeCommands,
    },
}

#[derive(Subcommand)]
pub enum AiCodeCommands {
    Explain {
        file: String,
        #[arg(long)]
        json: bool,
    },
    Deps {
        file: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum AiToolsCommands {
    List {
        #[arg(long)]
        json: bool,
    },
    Inspect {
        name: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum AiMcpCommands {
    Serve {
        #[arg(long)]
        stdio: bool,
    },
    Tools,
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
pub enum DistributeCommands {
    Plan(FleetConfigArgs),
    Status {
        #[arg(long, default_value = "cocli.recipe.toml")]
        config: String,
    },
    Explain(FleetConfigArgs),
    Run(DistributeRunArgs),
    Resume(FleetConfigArgs),
    Clean,
    Recipe {
        #[command(subcommand)]
        command: DistributeRecipeCommands,
    },
    Pool {
        #[command(subcommand)]
        command: DistributePoolCommands,
    },
    Shard {
        #[command(subcommand)]
        command: DistributeShardCommands,
    },
}

#[derive(clap::Args, Clone)]
pub struct DistributeRunArgs {
    #[arg(long, default_value = "cocli.recipe.toml")]
    pub config: String,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long)]
    pub cost: bool,
    #[arg(long)]
    pub allow_fallback_account: bool,
}

#[derive(Subcommand)]
pub enum DistributeRecipeCommands {
    Init {
        #[arg(long, default_value = "cocli.recipe.toml")]
        out: String,
    },
    Check(FleetConfigArgs),
    Explain(FleetConfigArgs),
    Run(DistributeRunArgs),
}

#[derive(Subcommand)]
pub enum DistributePoolCommands {
    Plan(FleetConfigArgs),
    Status {
        #[arg(long, default_value = "cocli.recipe.toml")]
        config: String,
    },
    Cost(FleetConfigArgs),
    Logs,
}

#[derive(Subcommand)]
pub enum DistributeShardCommands {
    Plan(FleetConfigArgs),
    Run(DistributeRunArgs),
    Resume(FleetConfigArgs),
}

#[derive(Subcommand)]
pub enum ReleaseCommands {
    Name {
        version: Option<String>,
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
        session: Option<String>,
        #[arg(long)]
        name: Option<String>,
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
        #[arg(long)]
        confirm: bool,
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
            "colab", "session", "new", "--name", "trainer", "--gpu", "A100",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Session {
                command: Some(SessionCommands::New(_))
            })
        ));
    }

    #[test]
    fn run_script_parses_forwarded_args() {
        let cli = Cli::try_parse_from([
            "colab",
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
        let Some(Commands::Run {
            command:
                RunCommands::Script {
                    script,
                    session,
                    ast: _,
                    args,
                },
        }) = cli.command
        else {
            panic!("expected run script");
        };
        assert_eq!(script, "train.py");
        assert_eq!(session.as_deref(), Some("trainer"));
        assert_eq!(args, ["--epochs", "3"]);
    }

    #[test]
    fn fs_pull_and_compat_download_parse() {
        assert!(Cli::try_parse_from(["colab", "fs", "pull", "/content/out", "./out"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "download", "/content/out", "./out"]).is_ok());
    }

    #[test]
    fn new_followup_spaces_parse() {
        assert!(Cli::try_parse_from(["colab", "run", "pip", "install", "torch"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "run", "pip", "check"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "run", "pip", "list"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "run", "ast", "file.py"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "run", "watch", "file.py", "--ast"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "run", "install", "torch"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "auth", "login", "--method", "adc"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "auth", "login", "--method", "oauth2"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "auth", "status"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "auth", "list"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "refresh"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "repair"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "reconnect"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "logs", "--tail", "20"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "kernel", "status"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "session", "kernel", "restart", "--yes"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "status", "runtime", "--gpu"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "status", "version"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "fs", "drive", "mount"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "colab",
                "fs",
                "drive",
                "mount",
                "--preflight-timeout",
                "10",
                "--retries",
                "2"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "colab",
                "session",
                "new",
                "--shape",
                "standard",
                "--retries",
                "2"
            ])
            .is_ok()
        );
        assert!(Cli::try_parse_from(["colab", "session", "new", "--no-retry"]).is_ok());
        assert!(
            Cli::try_parse_from(["colab", "settings", "skills", "inspect", "recipe.plan"]).is_ok()
        );
        assert!(Cli::try_parse_from(["colab", "settings", "experiments"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "settings", "about"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "settings", "update", "check"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "settings", "billing", "open", "--dry-run"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "settings", "experiments", "get"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "colab",
                "settings",
                "experiments",
                "set",
                "distribute",
                "true"
            ])
            .is_ok()
        );
        assert!(Cli::try_parse_from(["colab", "settings", "experiments", "reset"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "distribute", "plan"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "fleet", "plan", "--config", "slurp.toml"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "slurp", "explain"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "tools", "list"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "tools", "inspect", "recipe.plan"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "ast", "file.py"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "ast", "watch", "file.py"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "mcp"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "mcp", "serve", "--stdio"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "plan", "train"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "ai", "audit", "plan.toml"]).is_ok());
        assert!(Cli::try_parse_from(["colab", "release", "name", "v0.4.2"]).is_err());
        #[cfg(any(feature = "dev-tools", feature = "owner-tools"))]
        assert!(
            Cli::try_parse_from(["colab", "settings", "dev", "release", "name", "v0.4.2"]).is_ok()
        );
    }

    #[test]
    fn help_subcommand_is_disabled() {
        let Err(err) = Cli::try_parse_from(["colab", "help"]) else {
            panic!("`colab help` should not parse");
        };
        assert!(matches!(
            err.kind(),
            clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument
        ));
    }
}
