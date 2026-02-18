use anyhow::Context;
use clap::{CommandFactory, Parser, Subcommand};

use crate::config::{Config, load_config, run_onboarding};
use crate::providers::ProviderError;

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

    /// Model to use (overrides config file and environment)
    #[arg(long, global = true, value_name = "MODEL")]
    pub model: Option<String>,

    /// Path to config file (overrides default ~/.miniclaw/config.json)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<std::path::PathBuf>,

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

    /// Send a single message to the agent
    ///
    /// Executes a one-shot interaction with the agent, processing the message
    /// and returning the response without running the full gateway daemon.
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```bash
    /// miniclaw agent -m "Hello, how are you?"
    /// ```
    ///
    /// With model override:
    /// ```bash
    /// miniclaw agent -M "google/gemini-2.5-flash" -m "Explain Rust ownership"
    /// ```
    ///
    /// With verbose mode:
    /// ```bash
    /// miniclaw --verbose agent -m "Debug this code"
    /// ```
    Agent {
        /// Message to send to the agent
        #[arg(short, long, help = "Message to send to the agent")]
        message: String,

        /// Model to use for this request (overrides config)
        #[arg(short = 'M', long, help = "Model to use for this request")]
        model: Option<String>,
    },

    /// Memory management commands
    ///
    /// Read and manage long-term memory stored in MEMORY.md.
    ///
    /// # Examples
    ///
    /// Read today's entries (default):
    /// ```bash
    /// miniclaw memory read
    /// ```
    ///
    /// Read all historical entries:
    /// ```bash
    /// miniclaw memory read --long
    /// ```
    ///
    /// Read today's entries explicitly:
    /// ```bash
    /// miniclaw memory read --today
    /// ```
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },

    /// Start the gateway daemon
    ///
    /// Runs miniclaw as a background daemon with automatic session persistence.
    /// The gateway manages the ChatHub for message routing and persists sessions
    /// to disk every 30 seconds.
    ///
    /// # Examples
    ///
    /// Start the gateway:
    /// ```bash
    /// miniclaw gateway
    /// ```
    ///
    /// Start with verbose logging:
    /// ```bash
    /// miniclaw --verbose gateway
    /// ```
    ///
    /// Start with PID file for systemd:
    /// ```bash
    /// miniclaw gateway --pid-file /run/miniclaw.pid
    /// ```
    Gateway {
        /// Path to PID file (for systemd/docker)
        #[arg(long, value_name = "PATH")]
        pid_file: Option<std::path::PathBuf>,
    },

    /// List available models from the configured provider
    ///
    /// Displays all available models from the current provider, sorted alphabetically.
    /// Models marked as deprecated will show a [deprecated] indicator.
    ///
    /// # Examples
    ///
    /// List models:
    /// ```bash
    /// miniclaw models
    /// ```
    ///
    /// With custom provider:
    /// ```bash
    /// miniclaw --config /path/to/config.json models
    /// ```
    Models,
}

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Read memory entries
    Read {
        /// Show today's entries only (default)
        #[arg(long, group = "filter")]
        today: bool,

        /// Show all historical entries
        #[arg(long, group = "filter")]
        long: bool,
    },
    /// Read recent daily notes
    ///
    /// Displays daily notes from the last N days, sorted chronologically.
    ///
    /// # Examples
    ///
    /// Show last 7 days (default):
    /// ```bash
    /// miniclaw memory recent
    /// ```
    ///
    /// Show last 14 days:
    /// ```bash
    /// miniclaw memory recent --days 14
    /// miniclaw memory recent -d 14
    /// ```
    Recent {
        /// Number of days to show (default: 7)
        #[arg(short, long, default_value = "7", value_name = "N")]
        days: usize,
    },

    /// Search memories by relevance
    ///
    /// Searches across long-term memory and daily notes using keyword matching.
    /// Results are ranked by relevance score and displayed with excerpts.
    ///
    /// # Examples
    ///
    /// Basic search:
    /// ```bash
    /// miniclaw memory rank -q "project meeting"
    /// ```
    ///
    /// Search with custom limit:
    /// ```bash
    /// miniclaw memory rank -q "architecture" -n 10
    /// ```
    Rank {
        /// Search query
        #[arg(short, long, required = true, value_name = "QUERY")]
        query: String,

        /// Maximum number of results (default: 5)
        #[arg(short = 'n', long, default_value = "5", value_name = "N")]
        limit: usize,
    },
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    tracing::debug!("CLI parsing complete, processing command");

    let config = load_config(cli.model.clone(), cli.config.clone())
        .context("Error loading configuration")?;

    match cli.command {
        Some(Commands::Version) => {
            tracing::debug!("Executing version command");
            print_version(&config);
            Ok(())
        }
        Some(Commands::Onboard { yes, path }) => {
            tracing::debug!("Executing onboard command");
            handle_onboard(yes, path, &config)?;
            Ok(())
        }
        Some(Commands::Help { command }) => {
            tracing::debug!("Executing help command");
            handle_help(command)?;
            Ok(())
        }
        Some(Commands::Agent { message, model }) => {
            tracing::debug!("Executing agent command");
            handle_agent(message, model, &config, cli.verbose)
        }
        Some(Commands::Memory { command }) => {
            tracing::debug!("Executing memory command");
            handle_memory_command(command, &config)
        }
        Some(Commands::Gateway { pid_file }) => {
            tracing::debug!("Executing gateway command");
            handle_gateway(&config, pid_file)
        }
        Some(Commands::Models) => {
            tracing::debug!("Executing models command");
            handle_models(&config)
        }
        None => {
            tracing::debug!("No subcommand provided, showing help");
            let mut cmd = Cli::command();
            cmd.print_help().unwrap();
            println!();
            Ok(())
        }
    }
}

fn print_version(config: &Config) {
    tracing::debug!("Displaying version information");
    println!("miniclaw {}", env!("CARGO_PKG_VERSION"));
    if let Some(provider_config) = &config.provider_config {
        println!("Default model: {}", provider_config.default_model());
    }
}

fn handle_onboard(yes: bool, path: Option<String>, config: &Config) -> anyhow::Result<()> {
    tracing::info!(yes = yes, path = ?path, "Starting onboard command");

    run_onboarding(
        config.provider_config.is_some() || std::env::var("RUST_LOG").is_ok(),
        path,
        yes,
    )
}

fn handle_agent(
    message: String,
    model: Option<String>,
    config: &Config,
    verbose: bool,
) -> anyhow::Result<()> {
    use crate::agent::execute_one_shot;

    tracing::info!(message = %message, model = ?model, "Starting agent one-shot command");

    // Create a tokio runtime for the async execution
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    // Execute the one-shot command
    let result = rt.block_on(async { execute_one_shot(message, model, config, verbose).await });

    // Explicitly shutdown the runtime to ensure clean resource cleanup
    rt.shutdown_timeout(std::time::Duration::from_secs(5));

    match result {
        Ok(response) => {
            // Print the response to stdout
            println!("{}", response);
            Ok(())
        }
        Err(e) => {
            // Print error to stderr and return error for proper exit code
            eprintln!("Error: {}", e);
            Err(e)
        }
    }
}

fn handle_memory_command(command: MemoryCommands, config: &Config) -> anyhow::Result<()> {
    tracing::info!("Starting memory command");

    // Create a tokio runtime for the async execution
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    let result = rt.block_on(async {
        match command {
            MemoryCommands::Read { today, long } => handle_memory_read(today, long, config).await,
            MemoryCommands::Recent { days } => handle_memory_recent(days, config).await,
            MemoryCommands::Rank { query, limit } => handle_memory_rank(query, limit, config).await,
        }
    });

    rt.shutdown_timeout(std::time::Duration::from_secs(5));

    result
}

async fn handle_memory_read(today: bool, long: bool, _config: &Config) -> anyhow::Result<()> {
    use crate::memory::MemoryStore;

    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let memory_store = MemoryStore::new(workspace_path);

    // Default to showing today if neither flag is set
    let show_today = today || !long;

    if show_today {
        // Show today's entries with formatting
        let entries = memory_store
            .long_term()
            .read_today()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read memory: {}", e))?;

        if entries.is_empty() {
            println!("\x1b[33m📝 No entries for today.\x1b[0m");
        } else {
            let today = chrono::Utc::now().format("%Y-%m-%d");
            println!("\x1b[1;36m## 📅 Today's Memory ({})\x1b[0m\n", today);

            for (i, entry) in entries.iter().enumerate() {
                let time = entry.timestamp.format("%H:%M:%S UTC");
                println!(
                    "\x1b[32m{:2}.\x1b[0m {} \x1b[90m({})\x1b[0m",
                    i + 1,
                    entry.content,
                    time
                );
            }

            println!("\n\x1b[90mTotal: {} entries\x1b[0m", entries.len());
        }
    } else if long {
        // Show all entries with pagination
        let sections = memory_store
            .long_term()
            .read_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read memory: {}", e))?;

        if sections.is_empty() {
            println!("\x1b[33m📝 No memory entries found.\x1b[0m");
        } else {
            let total_entries: usize = sections.iter().map(|s| s.entries.len()).sum();
            println!("\x1b[1;36m## 📚 All Memory Entries\x1b[0m");
            println!(
                "\x1b[90m{} sections, {} total entries\x1b[0m\n",
                sections.len(),
                total_entries
            );

            let mut entry_count = 0;
            for section in &sections {
                println!("\x1b[1;35m## 📅 {}\x1b[0m", section.date);

                for entry in &section.entries {
                    entry_count += 1;
                    let time = entry.timestamp.format("%H:%M:%S UTC");
                    println!(
                        "  \x1b[32m•\x1b[0m {} \x1b[90m({})\x1b[0m",
                        entry.content, time
                    );

                    // Simple pagination: pause every 20 entries
                    if entry_count % 20 == 0 && entry_count < total_entries {
                        println!(
                            "\n\x1b[90m--- Press Enter to continue ({}/{} entries) ---\x1b[0m",
                            entry_count, total_entries
                        );
                        let mut buffer = String::new();
                        std::io::stdin().read_line(&mut buffer).ok();
                    }
                }
                println!();
            }

            println!(
                "\x1b[90mTotal: {} entries across {} days\x1b[0m",
                total_entries,
                sections.len()
            );
        }
    }

    Ok(())
}

async fn handle_memory_recent(days: usize, _config: &Config) -> anyhow::Result<()> {
    use crate::memory::MemoryStore;

    // Validate days parameter
    if days == 0 {
        anyhow::bail!("Days must be a positive integer");
    }
    if days > 365 {
        eprintln!(
            "\x1b[33m⚠️  Warning: Large day count ({}). This may take a while.\x1b[0m",
            days
        );
    }

    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let memory_store = MemoryStore::new(workspace_path);

    tracing::info!(days = days, "Reading recent daily notes");

    // Read recent daily notes
    let sections = memory_store
        .read_recent_daily_notes(days)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read daily notes: {}", e))?;

    if sections.is_empty() {
        println!(
            "\x1b[33m📝 No daily notes found for the last {} days.\x1b[0m",
            days
        );
    } else {
        let total_entries: usize = sections.iter().map(|s| s.entries.len()).sum();

        let days_text = if days == 1 { "day" } else { "days" };
        println!(
            "\x1b[1;36m## 📅 Daily Notes (Last {} {})\x1b[0m",
            days, days_text
        );
        println!(
            "\x1b[90m{} sections, {} total entries\x1b[0m\n",
            sections.len(),
            total_entries
        );

        let mut entry_count = 0;
        for section in &sections {
            println!("\x1b[1;32m## 📅 {}\x1b[0m", section.date);

            for entry in &section.entries {
                entry_count += 1;
                let time = entry.timestamp.format("%H:%M:%S UTC");
                println!(
                    "  \x1b[32m•\x1b[0m {} \x1b[90m({})\x1b[0m",
                    entry.content, time
                );

                // Simple pagination: pause every 20 entries
                if entry_count % 20 == 0 && entry_count < total_entries {
                    println!(
                        "\n\x1b[90m--- Press Enter to continue ({}/{} entries) ---\x1b[0m",
                        entry_count, total_entries
                    );
                    let mut buffer = String::new();
                    std::io::stdin().read_line(&mut buffer).ok();
                }
            }
            println!();
        }

        println!(
            "\x1b[90mTotal: {} entries across {} days\x1b[0m",
            total_entries,
            sections.len()
        );
    }

    Ok(())
}

async fn handle_memory_rank(query: String, limit: usize, _config: &Config) -> anyhow::Result<()> {
    use crate::memory::{DEFAULT_SEARCH_LIMIT, MAX_SEARCH_RESULTS, MemoryStore};

    // Validate limit
    let limit = if limit == 0 {
        DEFAULT_SEARCH_LIMIT // Default if user passes 0
    } else {
        limit.min(MAX_SEARCH_RESULTS) // Cap at maximum
    };

    // Validate query length (max 1000 chars to prevent abuse)
    if query.len() > 1000 {
        anyhow::bail!("Query too long (max 1000 characters)");
    }

    let workspace_path = dirs::home_dir()
        .map(|home| home.join(".miniclaw").join("workspace"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let memory_store = MemoryStore::new(workspace_path);

    // Sanitize query for logging (truncate and escape)
    let safe_query = if query.len() > 50 {
        format!("{}...", &query[..50])
    } else {
        query.clone()
    };
    tracing::info!(query_length = query.len(), limit = limit, safe_query = %safe_query, "Searching memories");

    // Search memories
    let results = memory_store
        .search_memories(&query, limit)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to search memories: {}", e))?;

    // Display results
    println!("\x1b[1;36m🔍 Search Results for: \"{}\"\x1b[0m\n", query);

    if results.is_empty() {
        println!("\x1b[33mNo memories found matching your query.\x1b[0m\n");
        println!("\x1b[90m💡 Try:\x1b[0m");
        println!("   \x1b[90m• Using broader search terms\x1b[0m");
        println!("   \x1b[90m• Checking different keywords\x1b[0m");
        println!("   \x1b[90m• Searching with fewer words\x1b[0m");
    } else {
        for (i, result) in results.iter().enumerate() {
            let rank = i + 1;
            let date_str = result
                .date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown date".to_string());

            // Choose icon based on source
            let icon = match result.source {
                crate::memory::MemorySource::LongTerm => "📅",
                crate::memory::MemorySource::DailyNote => "📅",
            };

            println!(
                "\x1b[1;32m{}.\x1b[0m \x1b[90m[Score: {}]\x1b[0m {} {} \x1b[90m({})\x1b[0m",
                rank, result.score, icon, date_str, result.source
            );
            println!("   \x1b[0m{}\x1b[0m\n", result.excerpt);
        }

        let result_word = if results.len() == 1 {
            "memory"
        } else {
            "memories"
        };
        println!(
            "\x1b[90mFound {} relevant {} (showing top {})\x1b[0m",
            results.len(),
            result_word,
            results.len().min(limit)
        );
    }

    Ok(())
}

fn handle_gateway(config: &Config, pid_file: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use crate::gateway::run_gateway;

    tracing::info!("Starting gateway daemon command");

    // Write PID file if specified (for systemd compatibility)
    if let Some(ref path) = pid_file {
        let pid = std::process::id();
        std::fs::write(path, pid.to_string())
            .with_context(|| format!("Failed to write PID file to {}", path.display()))?;
        tracing::info!(pid = pid, path = %path.display(), "PID file written");
    }

    // Create a tokio runtime for the async execution
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    // Execute the gateway
    let result = rt.block_on(async { run_gateway(config).await });

    // Explicitly shutdown the runtime to ensure clean resource cleanup
    rt.shutdown_timeout(std::time::Duration::from_secs(10));

    // Clean up PID file on exit (if it was created)
    if let Some(ref path) = pid_file {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(path) {
                tracing::warn!(path = %path.display(), error = %e, "Failed to remove PID file");
            } else {
                tracing::info!(path = %path.display(), "PID file removed");
            }
        }
    }

    result
}

fn handle_models(config: &Config) -> anyhow::Result<()> {
    use crate::providers::ProviderFactory;

    tracing::info!("Starting models command");

    // Check if provider is configured
    let provider_config = config
        .provider_config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No provider configured. Run 'miniclaw onboard' first."))?;

    // Create provider
    let provider = ProviderFactory::create(provider_config.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create provider: {}", e))?;

    // Create a tokio runtime for the async execution
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    let result = rt.block_on(async {
        // List models
        let models = provider.list_models().await.map_err(|e| match e {
            ProviderError::Auth { message } => {
                anyhow::anyhow!("Authentication error: {}", message)
            }
            ProviderError::Network { message } => {
                anyhow::anyhow!("Network error: {}", message)
            }
            _ => anyhow::anyhow!("Failed to list models: {}", e),
        })?;

        // Display models
        display_models(&models, provider.provider_name());

        Ok::<(), anyhow::Error>(())
    });

    rt.shutdown_timeout(std::time::Duration::from_secs(5));

    result
}

fn display_models(models: &[crate::providers::ModelInfo], provider_name: &str) {
    if models.is_empty() {
        println!("\x1b[33mNo models available for {}\x1b[0m", provider_name);
        return;
    }

    println!(
        "\x1b[1;36m## Available models for {}\x1b[0m\n",
        provider_name
    );

    for model in models {
        if model.deprecated {
            println!("\x1b[90m• {} [deprecated]\x1b[0m", model.id);
        } else {
            println!("\x1b[32m•\x1b[0m {}", model.id);
        }
    }

    println!("\n\x1b[90mTotal: {} model(s)\x1b[0m", models.len());
}

pub fn handle_help(command: Option<String>) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    match command {
        Some(cmd_name) => {
            if cmd.find_subcommand(&cmd_name).is_some() {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .arg(&cmd_name)
                    .arg("--help")
                    .status()
                    .context("Failed to execute help command")?;
                Ok(())
            } else {
                anyhow::bail!("Unknown command: {}", cmd_name);
            }
        }
        None => {
            cmd.print_help().unwrap();
            println!();
            Ok(())
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

    #[test]
    fn test_agent_command_parsing() {
        let cli = Cli::parse_from(["miniclaw", "agent", "-m", "Hello"]);
        assert!(!cli.verbose);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: None
            }) if message == "Hello"
        ));
    }

    #[test]
    fn test_agent_with_long_message_flag() {
        let cli = Cli::parse_from(["miniclaw", "agent", "--message", "Test message"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: None
            }) if message == "Test message"
        ));
    }

    #[test]
    fn test_agent_with_model_override() {
        let cli = Cli::parse_from(["miniclaw", "agent", "-M", "custom-model", "-m", "Hello"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: Some(m)
            }) if message == "Hello" && m == "custom-model"
        ));
    }

    #[test]
    fn test_agent_with_long_model_flag() {
        let cli = Cli::parse_from([
            "miniclaw",
            "agent",
            "--model",
            "google/gemini-2.5-flash",
            "--message",
            "Test",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: Some(m)
            }) if message == "Test" && m == "google/gemini-2.5-flash"
        ));
    }

    #[test]
    fn test_agent_command_with_verbose() {
        let cli = Cli::parse_from(["miniclaw", "--verbose", "agent", "-m", "Hello"]);
        assert!(cli.verbose);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: None
            }) if message == "Hello"
        ));
    }

    #[test]
    fn test_agent_complex_message() {
        let cli = Cli::parse_from(["miniclaw", "agent", "-m", "What is 2 + 2?"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Agent {
                message,
                model: None
            }) if message == "What is 2 + 2?"
        ));
    }

    #[test]
    fn test_memory_rank_command_parsing() {
        let cli = Cli::parse_from(["miniclaw", "memory", "rank", "-q", "project meeting"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Memory {
                command: MemoryCommands::Rank { query, limit: 5 }
            }) if query == "project meeting"
        ));
    }

    #[test]
    fn test_memory_rank_with_long_query_flag() {
        let cli = Cli::parse_from([
            "miniclaw",
            "memory",
            "rank",
            "--query",
            "architecture review",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Memory {
                command: MemoryCommands::Rank { query, limit: 5 }
            }) if query == "architecture review"
        ));
    }

    #[test]
    fn test_memory_rank_with_limit() {
        let cli = Cli::parse_from(["miniclaw", "memory", "rank", "-q", "test", "-n", "10"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Memory {
                command: MemoryCommands::Rank { query, limit: 10 }
            }) if query == "test"
        ));
    }

    #[test]
    fn test_memory_rank_with_long_limit_flag() {
        let cli = Cli::parse_from([
            "miniclaw", "memory", "rank", "--query", "test", "--limit", "3",
        ]);
        assert!(matches!(
            cli.command,
            Some(Commands::Memory {
                command: MemoryCommands::Rank { query, limit: 3 }
            }) if query == "test"
        ));
    }
}
