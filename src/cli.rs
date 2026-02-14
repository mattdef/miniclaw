use clap::{Parser, Subcommand};
use std::env;
use std::process;

#[derive(Parser)]
#[command(name = "miniclaw")]
#[command(about = "miniclaw - AI Agent CLI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,
}

pub fn run() {
    // Check for version flags manually before parsing
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        print_version();
        process::exit(0);
    }

    // Check if no args provided - show help
    if args.len() <= 1 {
        print_help();
        process::exit(0);
    }

    match Cli::try_parse() {
        Ok(cli) => {
            match cli.command {
                Some(Commands::Version) => {
                    print_version();
                    process::exit(0);
                }
                None => {
                    // No subcommand provided - show help
                    print_help();
                    process::exit(0);
                }
            }
        }
        Err(e) => {
            // Handle clap errors to set appropriate exit codes
            let error_message = e.to_string();

            // Check if this is an unknown subcommand error
            if error_message.contains("unrecognized subcommand")
                || error_message.contains("error: unrecognized subcommand")
                || error_message.contains("unknown command")
            {
                let cmd = extract_command_from_error(&error_message);
                eprintln!("error: unknown command: {}", cmd);
                eprintln!("\nValid commands are: version, help");
                process::exit(1);
            }

            // Check if this is an invalid argument/flag error
            if error_message.contains("unexpected argument")
                || error_message.contains("found argument")
                || error_message.contains("error: unexpected argument")
            {
                e.print().ok();
                process::exit(2);
            }

            // Default error handling
            e.print().ok();
            process::exit(2);
        }
    }
}

fn extract_command_from_error(error_msg: &str) -> String {
    // Try to extract the command name from error messages like:
    // "error: unrecognized subcommand 'invalid_command'"
    if let Some(start) = error_msg.find('\'') {
        if let Some(end) = error_msg[start + 1..].find('\'') {
            return error_msg[start + 1..start + 1 + end].to_string();
        }
    }
    "unknown".to_string()
}

fn print_version() {
    println!("miniclaw {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!("miniclaw - AI Agent CLI");
    println!();
    println!("Usage: miniclaw <COMMAND>");
    println!();
    println!("Commands:");
    println!("  version  Display version information");
    println!("  help     Print this message or the help of the given subcommand(s)");
    println!();
    println!("Options:");
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
    fn test_extract_command_from_error() {
        let msg = "error: unrecognized subcommand 'foo_bar'";
        assert_eq!(extract_command_from_error(msg), "foo_bar");

        let msg2 = "unrecognized subcommand 'test'";
        assert_eq!(extract_command_from_error(msg2), "test");
    }

    #[test]
    fn test_extract_command_from_error_unknown() {
        let msg = "some error without command";
        assert_eq!(extract_command_from_error(msg), "unknown");
    }

    #[test]
    fn test_print_help_shows_commands() {
        let mut buf = Vec::new();
        {
            let mut out = std::io::BufWriter::new(&mut buf);
            writeln!(out, "miniclaw - AI Agent CLI").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "Usage: miniclaw <COMMAND>").unwrap();
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
