use crate::config::{Config, save_config};
use crate::workspace;
use anyhow::{Context, Result};
use inquire::Confirm;
use inquire::Text;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_onboarding(
    verbose: bool,
    custom_path: Option<String>,
    skip_prompts: bool,
) -> Result<()> {
    let base_path = determine_base_path(custom_path)?;

    if verbose {
        tracing::debug!(path = %base_path.display(), "Starting onboarding with custom path");
    }

    let workspace_exists = base_path.exists();

    if workspace_exists {
        let should_reconfigure = handle_reconfiguration(&base_path, skip_prompts, verbose)?;
        if !should_reconfigure {
            return Ok(());
        }
    }

    create_workspace_structure(&base_path, verbose)?;

    let config = collect_user_configuration(skip_prompts, verbose)?;

    save_configuration(&base_path, &config, verbose)?;

    display_completion_summary(&config, verbose);

    Ok(())
}

fn determine_base_path(custom_path: Option<String>) -> Result<PathBuf> {
    if let Some(path) = custom_path {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(anyhow::anyhow!(
                "Invalid path: '{}'. Please provide an absolute path.",
                path.display()
            ));
        }
        Ok(path)
    } else {
        dirs::home_dir()
            .map(|p| p.join(".miniclaw"))
            .context("Could not determine home directory")
    }
}

fn handle_reconfiguration(base_path: &Path, skip_prompts: bool, verbose: bool) -> Result<bool> {
    if verbose {
        tracing::debug!("Workspace already exists, checking for reconfiguration");
    }

    println!("Workspace already exists at {}", base_path.display());

    let reconfigure = if skip_prompts {
        false
    } else {
        Confirm::new("Do you want to reconfigure? (y/N)")
            .with_default(false)
            .with_help_message("Press 'y' to overwrite existing configuration, 'n' to preserve it")
            .prompt()?
    };

    if !reconfigure {
        println!("Preserving existing workspace configuration.");
        return Ok(false);
    }

    println!("Proceeding with reconfiguration...");
    Ok(true)
}

fn create_workspace_structure(base_path: &PathBuf, verbose: bool) -> Result<()> {
    if verbose {
        tracing::debug!(path = %base_path.display(), "Creating workspace directory structure");
    }

    // Create the base miniclaw directory
    fs::create_dir_all(base_path).context("Failed to create miniclaw directory")?;
    if verbose {
        println!("Created directory: {}", base_path.display());
    }

    // Initialize the workspace with all markdown files
    workspace::initialize_workspace(base_path, verbose)
        .context("Failed to initialize workspace")?;

    println!();
    println!("Workspace initialized successfully");

    Ok(())
}

fn collect_user_configuration(skip_prompts: bool, verbose: bool) -> Result<Config> {
    let mut config = Config::default();

    if skip_prompts {
        if verbose {
            tracing::debug!("Skipping prompts, using default configuration");
        }
        return Ok(config);
    }

    if verbose {
        tracing::debug!("Starting interactive configuration prompts");
    }

    println!();
    println!("Let's configure miniclaw!");

    // Prompt for provider selection first
    config.provider_config = prompt_provider_selection(verbose)?;

    config.telegram_token = prompt_telegram_token(verbose)?;

    // Configure whitelist if Telegram token is set
    if config.telegram_token.is_some() {
        config.allow_from = prompt_telegram_whitelist(verbose)?;
    }

    // Confirmation step before saving (AC 4)
    if !confirm_configuration(&config)? {
        println!("Configuration cancelled. No changes were made.");
        return Ok(Config::default());
    }

    Ok(config)
}

fn confirm_configuration(config: &Config) -> Result<bool> {
    println!();
    println!("Please review your configuration:");
    println!();
    println!("{}", format_provider_summary(config));

    if let Some(token) = &config.telegram_token {
        println!("  Telegram Bot: {}", mask_secret(token));
    } else {
        println!("  Telegram Bot: (not set)");
    }

    if config.allow_from.is_empty() {
        println!("  Whitelisted Users: (empty - secure by default, no access)");
    } else {
        let user_ids: Vec<String> = config.allow_from.iter().map(|id| id.to_string()).collect();
        println!("  Whitelisted Users: {}", user_ids.join(", "));
    }

    println!();
    let confirmed = Confirm::new("Save this configuration?")
        .with_default(true)
        .with_help_message("Press 'y' to save, 'n' to cancel")
        .prompt()?;

    Ok(confirmed)
}

fn prompt_provider_selection(verbose: bool) -> Result<Option<crate::providers::ProviderConfig>> {
    use crate::providers::{
        KimiConfig, OllamaConfig, OpenAiConfig, OpenRouterConfig, ProviderConfig,
    };
    use inquire::Select;

    println!();
    println!("LLM Provider Selection");
    println!("Choose which LLM provider you want to use:");

    let options = vec![
        "OpenRouter (recommended - access to multiple models)",
        "OpenAI (native OpenAI API)",
        "Kimi (Moonshot AI)",
        "Ollama (local models - no API key needed)",
    ];

    let selection = Select::new("Select provider:", options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()?;

    match selection {
        "OpenRouter (recommended - access to multiple models)" => {
            println!("\nOpenRouter API Configuration");
            println!("Get one for free at: https://openrouter.ai/settings/keys");
            println!("Format: The key should start with 'sk-or-'");

            let api_key = Text::new("Enter your OpenRouter API key (or press Enter to skip):")
                .with_validator(|input: &str| {
                    if input.is_empty() || input.starts_with("sk-or-") {
                        Ok(inquire::validator::Validation::Valid)
                    } else {
                        Ok(inquire::validator::Validation::Invalid(
                            "API key must start with 'sk-or-' or be empty to skip".into(),
                        ))
                    }
                })
                .with_help_message("Press Enter without typing to skip this step")
                .prompt()?;

            if api_key.is_empty() {
                if verbose {
                    tracing::debug!("User skipped OpenRouter API key configuration");
                }
                return Ok(None);
            }

            let provider_config = ProviderConfig::OpenRouter(OpenRouterConfig::new(&api_key));
            Ok(Some(provider_config))
        }
        "OpenAI (native OpenAI API)" => {
            println!("\nOpenAI API Configuration");
            println!("Get your API key at: https://platform.openai.com/api-keys");
            println!("Format: The key should start with 'sk-'");

            let api_key = Text::new("Enter your OpenAI API key (or press Enter to skip):")
                .with_validator(|input: &str| {
                    if input.is_empty() || input.starts_with("sk-") {
                        Ok(inquire::validator::Validation::Valid)
                    } else {
                        Ok(inquire::validator::Validation::Invalid(
                            "API key must start with 'sk-' or be empty to skip".into(),
                        ))
                    }
                })
                .with_help_message("Press Enter without typing to skip this step")
                .prompt()?;

            if api_key.is_empty() {
                if verbose {
                    tracing::debug!("User skipped OpenAI API key configuration");
                }
                return Ok(None);
            }

            let provider_config = ProviderConfig::OpenAi(OpenAiConfig::new(&api_key));
            Ok(Some(provider_config))
        }
        "Kimi (Moonshot AI)" => {
            println!("\nKimi (Moonshot AI) API Configuration");
            println!("Get your API key at: https://platform.moonshot.cn");
            println!("Format: The key should start with 'sk-kimi-'");

            let api_key = Text::new("Enter your Kimi API key (or press Enter to skip):")
                .with_validator(|input: &str| {
                    if input.is_empty() || input.starts_with("sk-kimi-") {
                        Ok(inquire::validator::Validation::Valid)
                    } else {
                        Ok(inquire::validator::Validation::Invalid(
                            "API key must start with 'sk-kimi-' or be empty to skip".into(),
                        ))
                    }
                })
                .with_help_message("Press Enter without typing to skip this step")
                .prompt()?;

            if api_key.is_empty() {
                if verbose {
                    tracing::debug!("User skipped Kimi API key configuration");
                }
                return Ok(None);
            }

            let provider_config = ProviderConfig::Kimi(KimiConfig::new(&api_key));
            Ok(Some(provider_config))
        }
        "Ollama (local models - no API key needed)" => {
            println!("\nOllama Configuration");
            println!("Using local Ollama instance at http://localhost:11434");
            println!("Make sure Ollama is running locally.");

            let provider_config = ProviderConfig::Ollama(OllamaConfig::new());
            Ok(Some(provider_config))
        }
        _ => Ok(None),
    }
}

fn prompt_telegram_token(verbose: bool) -> Result<Option<String>> {
    println!();
    println!("Telegram Bot Configuration");
    println!("To enable Telegram integration, follow these steps:");
    println!("  Step 1: Open Telegram and message @BotFather");
    println!("  Step 2: Type /newbot and follow the instructions");
    println!("  Step 3: Copy the bot token here");

    let token = Text::new("Enter your Telegram bot token (or press Enter to skip):")
        .with_validator(|input: &str| {
            if input.is_empty() || validate_telegram_token(input) {
                Ok(inquire::validator::Validation::Valid)
            } else {
                Ok(inquire::validator::Validation::Invalid(
                    "Invalid token format. Should look like: 1234567890:ABCdefGHIjklMNOpqrsTUVwxyz"
                        .into(),
                ))
            }
        })
        .with_help_message("Press Enter without typing to skip this step")
        .prompt()?;

    if token.is_empty() {
        if verbose {
            tracing::debug!("User skipped Telegram token configuration");
        }
        return Ok(None);
    }

    if verbose {
        tracing::debug!(masked = true, "Telegram token configured");
    }

    Ok(Some(token))
}

fn validate_telegram_token(token: &str) -> bool {
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].chars().all(|c| c.is_ascii_digit()) && parts[1].len() >= 30
}

fn prompt_telegram_whitelist(verbose: bool) -> Result<Vec<i64>> {
    println!();
    println!("📱 Configuration Telegram - Sécurité");
    println!("Pour sécuriser votre agent, vous devez spécifier qui peut interagir avec lui.");
    println!();
    println!("Pour trouver votre ID Telegram :");
    println!("  1. Ouvrez Telegram et cherchez @userinfobot");
    println!("  2. Cliquez sur 'Démarrer'");
    println!("  3. Le bot affichera votre ID (ex: 123456789)");
    println!("  4. Copiez cet ID ci-dessous");

    let user_id_input = Text::new("Votre ID Telegram:")
        .with_validator(|input: &str| {
            if input.is_empty() {
                Ok(inquire::validator::Validation::Valid)
            } else {
                match input.parse::<i64>() {
                    Ok(id) if id > 0 => Ok(inquire::validator::Validation::Valid),
                    _ => Ok(inquire::validator::Validation::Invalid(
                        "L'ID doit être un nombre positif (ex: 123456789)".into(),
                    )),
                }
            }
        })
        .with_help_message("Entrez votre ID Telegram ou appuyez sur Entrée pour ignorer")
        .prompt()?;

    if user_id_input.is_empty() {
        if verbose {
            tracing::debug!("User skipped Telegram whitelist configuration");
        }
        println!(
            "\n⚠️ Aucun ID fourni. La whitelist est vide - personne ne pourra accéder à l'agent."
        );
        return Ok(vec![]);
    }

    let user_id = user_id_input.parse::<i64>().unwrap_or(0);

    if verbose {
        tracing::debug!(user_id = %user_id, "Telegram whitelist configured");
    }

    println!("\n✅ ID {} ajouté à la whitelist", user_id);
    println!("Seuls les utilisateurs dans cette liste pourront interagir avec l'agent.");
    println!("Vous pouvez ajouter d'autres utilisateurs en modifiant ~/.miniclaw/config.json");

    Ok(vec![user_id])
}

fn save_configuration(base_path: &Path, config: &Config, verbose: bool) -> Result<()> {
    // Skip if no configuration to save (user cancelled)
    if config.provider_config.is_none()
        && config.telegram_token.is_none()
        && config.allow_from.is_empty()
    {
        if verbose {
            tracing::debug!("Skipping save - empty configuration (user cancelled)");
        }
        return Ok(());
    }

    let config_path = base_path.join("config.json");

    if verbose {
        tracing::debug!(path = %config_path.display(), "Saving configuration");
    }

    save_config(config, &config_path).context("Failed to save configuration")?;

    if verbose {
        println!("Configuration saved to: {}", config_path.display());

        // Show values in verbose mode
        println!("Saved values:");
        println!("{}", format_provider_summary(config));
        if let Some(token) = &config.telegram_token {
            println!("  Telegram Token: {}", mask_secret(token));
        }
        if !config.allow_from.is_empty() {
            let user_ids: Vec<String> = config.allow_from.iter().map(|id| id.to_string()).collect();
            println!("  Whitelisted Users: {}", user_ids.join(", "));
        }
    }

    Ok(())
}

fn display_completion_summary(config: &Config, verbose: bool) {
    println!();
    println!("Configuration Complete!");
    println!("{}", format_provider_summary(config));

    if let Some(token) = &config.telegram_token {
        println!("Telegram Bot: {}", mask_secret(token));
    } else {
        println!("Telegram Bot: (not set)");
    }

    if config.allow_from.is_empty() {
        println!("Whitelisted Users: (empty - secure by default)");
    } else {
        let user_ids: Vec<String> = config.allow_from.iter().map(|id| id.to_string()).collect();
        println!("Whitelisted Users: {}", user_ids.join(", "));
    }

    println!();
    println!("Next Steps:");
    println!("  Run 'miniclaw gateway' to start the agent");
    println!("  Or run 'miniclaw agent -m \"your message\"' for a single query");
    println!();
    println!("Tip: Run 'miniclaw onboard' again to reconfigure at any time.");

    if verbose {
        tracing::debug!("Onboarding completed successfully");
    }
}

/// Formats provider type and model as a human-readable summary string.
///
/// Returns lines suitable for printing in configuration display contexts.
/// When no provider is configured, returns a single "(not set)" line.
fn format_provider_summary(config: &Config) -> String {
    if let Some(provider_config) = &config.provider_config {
        format!(
            "  Provider: {}\n  Model: {}",
            provider_config.provider_type(),
            provider_config.default_model()
        )
    } else {
        "  Provider: (not set)".to_string()
    }
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 10 {
        "***".to_string()
    } else {
        let start = &secret[..6];
        let end = &secret[secret.len() - 4..];
        format!("{}...{}", start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_determine_base_path_with_custom() {
        let result = determine_base_path(Some("/tmp/test_miniclaw".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/test_miniclaw"));
    }

    #[test]
    fn test_determine_base_path_with_invalid() {
        let result = determine_base_path(Some("relative".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_telegram_token_valid() {
        assert!(validate_telegram_token(
            "1234567890:ABCdefGHIjklMNOpqrsTUVwxyz123456"
        ));
    }

    #[test]
    fn test_validate_telegram_token_invalid_no_colon() {
        assert!(!validate_telegram_token(
            "1234567890ABCdefGHIjklMNOpqrsTUVwxyz"
        ));
    }

    #[test]
    fn test_validate_telegram_token_invalid_short() {
        assert!(!validate_telegram_token("123:abc"));
    }

    #[test]
    fn test_mask_secret_short() {
        assert_eq!(mask_secret("short"), "***");
    }

    #[test]
    fn test_mask_secret_long() {
        let result = mask_secret("sk-or-1234567890abcdef");
        assert!(result.starts_with("sk-or-"));
        assert!(result.contains("..."));
        assert!(result.ends_with("cdef"));
    }

    #[test]
    fn test_create_workspace_structure_new() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_miniclaw");

        let result = create_workspace_structure(&path, false);

        assert!(result.is_ok());
        assert!(path.exists());
        assert!(path.join("workspace").exists());

        // Verify all workspace files are created
        let workspace = path.join("workspace");
        assert!(workspace.join("SOUL.md").exists());
        assert!(workspace.join("AGENTS.md").exists());
        assert!(workspace.join("USER.md").exists());
        assert!(workspace.join("TOOLS.md").exists());
        assert!(workspace.join("HEARTBEAT.md").exists());
    }

    #[test]
    fn test_collect_user_configuration_skip() {
        let result = collect_user_configuration(true, false);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.provider_config.is_none());
        assert!(config.telegram_token.is_none());
        assert!(config.allow_from.is_empty()); // Secure by default
    }
}
