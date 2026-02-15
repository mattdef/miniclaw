mod cli;
mod config;
mod session;
mod skills;
mod workspace;

use clap::Parser;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[allow(dead_code)]
fn init_logging(verbose: bool) {
    let filter_level = if verbose { Level::DEBUG } else { Level::INFO };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(filter_level.into()))
        .with_target(true) // Show module path
        .with_level(true) // Show log level
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_timer(tracing_subscriber::fmt::time::time())
        .init();
}

fn main() {
    // Parse CLI early to check verbose flag before full initialization
    match cli::Cli::try_parse() {
        Ok(cli) => {
            // Check for help subcommand before initializing logging
            if let Some(cli::Commands::Help { command }) = &cli.command {
                if let Err(e) = cli::handle_help(command.clone()) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
                std::process::exit(0);
            }

            // Initialize logging based on verbose flag
            init_logging(cli.verbose);

            tracing::debug!("Verbose mode enabled");
            tracing::info!("Starting miniclaw v{}", env!("CARGO_PKG_VERSION"));

            if let Err(e) = cli::run(cli) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            use clap::error::ErrorKind;

            match e.kind() {
                ErrorKind::DisplayVersion | ErrorKind::DisplayHelp => {
                    // Print version or help and exit successfully
                    // Clap handles this automatically
                    e.print().ok();
                    std::process::exit(0);
                }
                ErrorKind::UnknownArgument | ErrorKind::InvalidValue => {
                    // Invalid flags or arguments - exit with code 2
                    e.print().ok();
                    std::process::exit(2);
                }
                _ => {
                    // For other errors (unknown commands), use clap's output and exit with code 1
                    e.print().ok();
                    std::process::exit(1);
                }
            }
        }
    }
}
