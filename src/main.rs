use clap::Parser;
use miniclaw::cli;
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Records the startup instant as early as possible
static STARTUP_INSTANT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Records the startup instant. Call this as early as possible in main().
pub fn record_startup_start() {
    let _ = STARTUP_INSTANT.set(std::time::Instant::now());
}

/// Gets the startup duration from when `record_startup_start` was called.
/// Returns None if startup was not recorded.
pub fn get_startup_duration() -> Option<std::time::Duration> {
    STARTUP_INSTANT.get().map(|start| start.elapsed())
}

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
        .with_writer(std::io::stderr) // Output to stderr, not stdout
        .init();
}

fn main() {
    // Install panic handler FIRST to catch any early panics
    std::panic::set_hook(Box::new(|panic_info| {
        // Extract panic location
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        // Extract panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic message".to_string()
        };

        // Log to stderr with structured format
        eprintln!("PANIC at {}: {}", location, message);
        eprintln!("This is a bug. Please report it with the above information.");

        // If tracing is initialized, also log there
        // Note: This may fail if panic occurred during tracing initialization
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tracing::error!(
                location = %location,
                message = %message,
                "PANIC: Application panicked"
            );
        }));
    }));

    // Record startup time as early as possible
    record_startup_start();

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

            if cli.verbose {
                tracing::debug!("Verbose mode enabled");
            }
            tracing::debug!("Starting miniclaw v{}", env!("CARGO_PKG_VERSION"));

            // Log startup time (DEBUG level to avoid stderr noise on simple commands)
            if let Some(duration) = get_startup_duration() {
                tracing::debug!(startup_ms = duration.as_millis(), "Startup complete");
            }

            if let Err(e) = cli::run(cli) {
                tracing::error!("Application error: {}", e);
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
