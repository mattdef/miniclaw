use clap::{Parser, Subcommand};

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
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) | None => {
            print_version();
        }
    }
}

fn print_version() {
    println!("miniclaw {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {

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
}
