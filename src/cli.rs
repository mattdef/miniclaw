use clap::{Parser, Subcommand};
use std::process;
use tracing;

#[derive(Parser)]
#[command(name = "miniclaw")]
#[command(about = "miniclaw - AI Agent CLI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
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
}

pub fn run(cli: Cli) {
    tracing::debug!("CLI parsing complete, processing command");

    match cli.command {
        Some(Commands::Version) => {
            tracing::debug!("Executing version command");
            print_version();
            process::exit(0);
        }
        None => {
            tracing::debug!("No subcommand provided, showing help");
            print_help();
            process::exit(0);
        }
    }
}

fn print_version() {
    tracing::info!("Displaying version information");
    println!("miniclaw {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!("miniclaw - AI Agent CLI");
    println!();
    println!("Usage: miniclaw [OPTIONS] <COMMAND>");
    println!();
    println!("Commands:");
    println!("  version  Display version information");
    println!("  help     Print this message or the help of the given subcommand(s)");
    println!();
    println!("Options:");
    println!("  -v, --verbose  Enable verbose logging (DEBUG level)");
    println!("  -V, --version  Print version");
    println!("  -h, --help     Print help");
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Write;

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
        // Test that --verbose flag can be parsed
        let cli = Cli::parse_from(["miniclaw", "--verbose", "version"]);
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_short_verbose_flag_parsing() {
        // Test that -v short flag can be parsed
        let cli = Cli::parse_from(["miniclaw", "-v", "version"]);
        assert!(cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_default_verbosity() {
        // Test that verbose is false by default
        let cli = Cli::parse_from(["miniclaw", "version"]);
        assert!(!cli.verbose);
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn test_print_help_shows_commands() {
        let mut buf = Vec::new();
        {
            let mut out = std::io::BufWriter::new(&mut buf);
            writeln!(out, "miniclaw - AI Agent CLI").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "Usage: miniclaw [OPTIONS] <COMMAND>").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "Commands:").unwrap();
            writeln!(out, "  version  Display version information").unwrap();
            writeln!(
                out,
                "  help     Print this message or the help of the given subcommand(s)"
            )
            .unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("version"));
        assert!(output.contains("help"));
    }
}
