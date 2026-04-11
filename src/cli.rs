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

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Server lifecycle and access
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
    /// Remote file operations
    File {
        #[command(subcommand)]
        command: FileCommands,
    },
    /// Generate shell completions
    Completions { shell: clap_complete::Shell },
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Sign in to Google (opens browser)
    Login,
    /// Sign out and clear stored credentials
    Logout,
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Assign a new Colab server (interactive if no flags given)
    Assign {
        #[arg(long, value_parser = parse_variant)]
        variant: Option<crate::client::api::Variant>,

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
        variant: Option<crate::client::api::Variant>,

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
    ///
    /// The command and its args are sent to the runtime verbatim. Stdout/stderr
    /// stream back to the local terminal as the remote process produces them,
    /// and the remote exit status is propagated as this command's exit code.
    ///
    /// Examples:
    ///     colab-cli server run --name "Colab CPU" python -V
    ///     colab-cli server run ls -la /content
    ///     colab-cli server run bash -lc 'echo hi && uname -a'
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
    /// List files on the runtime (passes args through to remote `ls`)
    ///
    /// Examples:
    ///     colab-cli file ls
    ///     colab-cli file ls -lah /content
    ///     colab-cli file ls --name "Colab CPU" -a /tmp
    Ls {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `ls`. Defaults to `-lah /content`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Copy files on the runtime (passes args through to remote `cp`)
    ///
    /// Examples:
    ///     colab-cli file cp /content/foo /content/bar
    ///     colab-cli file cp -r /content/dir /content/dir2
    Cp {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `cp`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
    /// Remove files on the runtime (passes args through to remote `rm`)
    ///
    /// Examples:
    ///     colab-cli file rm /content/foo.txt
    ///     colab-cli file rm -rf /content/junk
    Rm {
        #[arg(long)]
        name: Option<String>,

        /// Args forwarded to the remote `rm`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
}

fn parse_variant(s: &str) -> std::result::Result<crate::client::api::Variant, String> {
    match s.to_ascii_lowercase().as_str() {
        "cpu" | "default" => Ok(crate::client::api::Variant::Cpu),
        "gpu" => Ok(crate::client::api::Variant::Gpu),
        "tpu" => Ok(crate::client::api::Variant::Tpu),
        other => Err(format!(
            "unknown variant '{other}' — expected cpu, gpu, or tpu"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::api::Variant;
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
    fn parse_variant_rejects_garbage() {
        assert!(parse_variant("fpga").is_err());
        assert!(parse_variant("").is_err());
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

    #[test]
    fn ls_available_flag_parses() {
        let cli = Cli::try_parse_from(["colab-cli", "server", "ls", "--available"]).unwrap();
        if let Commands::Server {
            command: ServerCommands::Ls { available },
        } = cli.command
        {
            assert!(available);
        } else {
            panic!("expected ls");
        }
    }

    #[test]
    fn assign_keepalive_and_high_ram_flags_parse() {
        let cli = Cli::try_parse_from([
            "colab-cli",
            "server",
            "assign",
            "-k",
            "--high-ram",
            "--variant",
            "gpu",
        ])
        .unwrap();
        if let Commands::Server {
            command:
                ServerCommands::Assign {
                    keepalive,
                    high_ram,
                    variant,
                    ..
                },
        } = cli.command
        {
            assert!(keepalive);
            assert!(high_ram);
            assert_eq!(variant, Some(Variant::Gpu));
        } else {
            panic!("expected assign");
        }
    }

    #[test]
    fn reconfigure_parses() {
        let cli = Cli::try_parse_from([
            "colab-cli",
            "server",
            "reconfigure",
            "--name",
            "box",
            "--variant",
            "gpu",
            "-a",
            "T4",
            "--high-ram",
            "-k",
        ])
        .unwrap();
        if let Commands::Server {
            command:
                ServerCommands::Reconfigure {
                    name,
                    variant,
                    accelerator,
                    high_ram,
                    keepalive,
                },
        } = cli.command
        {
            assert_eq!(name.as_deref(), Some("box"));
            assert_eq!(variant, Some(Variant::Gpu));
            assert_eq!(accelerator.as_deref(), Some("T4"));
            assert!(high_ram);
            assert!(keepalive);
        } else {
            panic!("expected reconfigure");
        }
    }

    #[test]
    fn no_standalone_keepalive_command() {
        assert!(Cli::try_parse_from(["colab-cli", "server", "keepalive"]).is_err());
    }

    #[test]
    fn no_standalone_accelerators_command() {
        assert!(Cli::try_parse_from(["colab-cli", "server", "accelerators"]).is_err());
    }

    #[test]
    fn ps_interval_parses() {
        let cli = Cli::try_parse_from(["colab-cli", "server", "ps", "--interval", "250"]).unwrap();
        if let Commands::Server {
            command: ServerCommands::Ps { interval, .. },
        } = cli.command
        {
            assert_eq!(interval, 250);
        } else {
            panic!("expected ps");
        }
    }
}
