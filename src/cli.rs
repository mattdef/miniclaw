use clap::{CommandFactory, Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "miniclaw")]
#[command(about = "Your AI agent for edge hardware")]
#[command(long_about = "A lightweight AI agent that runs on Raspberry Pi and edge devices.")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(propagate_version = true)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// Enable verbose logging (DEBUG level)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,

    /// Initialize workspace and configuration
    ///
    /// Creates the workspace structure with SOUL.md, AGENTS.md, USER.md,
    /// TOOLS.md, HEARTBEAT.md files and interactive configuration.
    ///
    /// # Examples
    ///
    /// Run with default settings:
    /// ```bash
    /// miniclaw onboard
    /// ```
    ///
    /// Skip prompts with yes flag:
    /// ```bash
    /// miniclaw onboard --yes
    /// miniclaw onboard -y
    /// ```
    ///
    /// Use custom path:
    /// ```bash
    /// miniclaw onboard --path /custom/path
    /// miniclaw onboard -p /custom/path
    /// ```
    Onboard {
        /// Skip interactive prompts and use defaults
        #[arg(short, long)]
        yes: bool,

        /// Custom workspace path [default: ~/.miniclaw]
        #[arg(short, long, value_name = "PATH")]
        path: Option<String>,
    },

    /// Print help for a command
    ///
    /// Use `miniclaw help <command>` to show help for a specific command.
    Help {
        /// The command to show help for
        #[arg(value_name = "COMMAND")]
        command: Option<String>,
    },
}

pub fn run(cli: Cli) {
    tracing::debug!("CLI parsing complete, processing command");

    match cli.command {
        Some(Commands::Version) => {
            tracing::debug!("Executing version command");
            print_version();
            process::exit(0);
        }
        Some(Commands::Onboard { yes, path }) => {
            tracing::debug!("Executing onboard command");
            handle_onboard(yes, path);
            process::exit(0);
        }
        Some(Commands::Help { command }) => {
            tracing::debug!("Executing help command");
            handle_help(command);
            process::exit(0);
        }
        None => {
            tracing::debug!("No subcommand provided, showing help");
            // Use clap's native help generation
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!(); // Add newline after help
            process::exit(0);
        }
    }
}

fn print_version() {
    tracing::info!("Displaying version information");
    println!("miniclaw {}", env!("CARGO_PKG_VERSION"));
}

fn handle_onboard(yes: bool, path: Option<String>) {
    tracing::info!(yes = yes, path = ?path, "Onboard command placeholder");
    println!("Onboard command - to be implemented");
    println!("Skip prompts: {}", yes);
    if let Some(p) = path {
        println!("Custom path: {}", p);
    }
}

pub fn handle_help(command: Option<String>) {
    let mut cmd = Cli::command();
    match command {
        Some(cmd_name) => {
            if cmd.find_subcommand(&cmd_name).is_some() {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .arg(&cmd_name)
                    .arg("--help")
                    .status()
                    .ok();
            } else {
                println!("Unknown command: {}", cmd_name);
                std::process::exit(1);
            }
        }
        None => {
            cmd.print_help().unwrap();
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string_format() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        // Semantic version format: major.minor.patch
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].parse::<u32>().is_ok());
        assert!(parts[1].parse::<u32>().is_ok());
        assert!(parts[2].parse::<u32>().is_ok());
    }

    #[test]
    fn test_verbose_flag_parsing() {
        let cli = Cli::parse_from(["miniclaw", "--verbose", "version"]);
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_short_verbose_flag_parsing() {
        let cli = Cli::parse_from(["miniclaw", "-v", "version"]);
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_default_verbosity() {
        let cli = Cli::parse_from(["miniclaw", "version"]);
        assert!(!cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_onboard_command_parsing() {
        let cli = Cli::parse_from(["miniclaw", "onboard"]);
        assert!(!cli.verbose);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                yes: false,
                path: None
            })
        ));
    }

    #[test]
    fn test_onboard_with_yes_flag() {
        let cli = Cli::parse_from(["miniclaw", "onboard", "--yes"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                yes: true,
                path: None
            })
        ));
    }

    #[test]
    fn test_onboard_with_path_flag() {
        let cli = Cli::parse_from(["miniclaw", "onboard", "--path", "/custom/path"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                yes: false,
                path: Some(_)
            })
        ));
        if let Some(Commands::Onboard { path: Some(p), .. }) = cli.command {
            assert_eq!(p, "/custom/path");
        }
    }

    #[test]
    fn test_onboard_with_short_flags() {
        let cli = Cli::parse_from(["miniclaw", "onboard", "-y", "-p", "/tmp/test"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                yes: true,
                path: Some(_)
            })
        ));
        if let Some(Commands::Onboard {
            yes: true,
            path: Some(p),
        }) = cli.command
        {
            assert_eq!(p, "/tmp/test");
        }
    }
}
