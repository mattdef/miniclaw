mod cli;

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
            // Initialize logging based on verbose flag
            init_logging(cli.verbose);

            tracing::debug!("Verbose mode enabled");
            tracing::info!("Starting miniclaw v{}", env!("CARGO_PKG_VERSION"));

            cli::run(cli);
        }
        Err(e) => {
            use clap::error::ErrorKind;

            match e.kind() {
                ErrorKind::DisplayVersion | ErrorKind::DisplayHelp => {
                    // Print version or help and exit successfully
                    e.print().ok();
                    std::process::exit(0);
                }
                _ => {
                    // Check if this is an unrecognized subcommand error
                    let error_message = e.to_string();
                    if error_message.contains("unrecognized subcommand")
                        || error_message.contains("error: unrecognized subcommand")
                    {
                        // Extract command name from error
                        if let Some(start) = error_message.find('\'') {
                            if let Some(end) = error_message[start + 1..].find('\'') {
                                let cmd = &error_message[start + 1..start + 1 + end];
                                eprintln!("error: unknown command: {}", cmd);
                                eprintln!("\nValid commands are: version, help");
                                std::process::exit(1);
                            }
                        }
                    }

                    // For other errors, use clap's default behavior
                    e.print().ok();
                    std::process::exit(2);
                }
            }
        }
    }
}
