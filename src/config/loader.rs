use crate::config::schema::Config;
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use thiserror::Error;

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static CONFIG_TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file contains invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn load_config(cli_model: Option<String>, cli_config_path: Option<PathBuf>) -> Result<Config> {
    tracing::debug!("Loading configuration");

    let mut config = Config::default();

    // Layer 1: Load from config file (~/.miniclaw/config.json)
    let config_file = cli_config_path.clone().or_else(get_default_config_path);

    if let Some(ref path) = config_file {
        if path.exists() {
            tracing::debug!(config_path = %path.display(), "Loading configuration from file");
            config = merge_config_from_file(config, path)?;

            let safe_summary = config.get_safe_summary();
            tracing::debug!(
                api_key_configured = safe_summary.api_key_configured,
                telegram_configured = safe_summary.telegram_configured,
                model = ?safe_summary.model,
                allow_from_count = safe_summary.allow_from_count,
                "Configuration loaded from file"
            );
        } else {
            tracing::debug!(config_path = %path.display(), "Config file not found, using defaults");
        }
    }

    // Layer 2: Environment variables override
    tracing::debug!("Applying environment variable overrides");
    config = merge_env_variables(config);

    // Layer 3: CLI flags override (highest precedence)
    if let Some(ref model) = cli_model {
        tracing::debug!(model = %model, "Applying CLI model override");
        apply_cli_model_override(&mut config, model);
    }

    if cli_config_path.is_some() {
        tracing::debug!("Applied CLI config path override");
    }

    let final_summary = config.get_safe_summary();
    tracing::debug!(
        api_key_configured = final_summary.api_key_configured,
        telegram_configured = final_summary.telegram_configured,
        model = ?final_summary.model,
        allow_from_count = final_summary.allow_from_count,
        spawn_log_output = final_summary.spawn_log_output,
        "Configuration loaded successfully"
    );

    Ok(config)
}

/// Apply CLI --model flag to override provider_config.default_model
fn apply_cli_model_override(config: &mut Config, model: &str) {
    use crate::providers::ProviderConfig;

    if let Some(ref mut provider_config) = config.provider_config {
        // Override the model in existing provider_config
        match provider_config {
            ProviderConfig::OpenRouter(cfg) => {
                cfg.default_model = model.to_string();
            }
            ProviderConfig::OpenAi(cfg) => {
                cfg.default_model = model.to_string();
            }
            ProviderConfig::Kimi(cfg) => {
                cfg.default_model = model.to_string();
            }
            ProviderConfig::Ollama(cfg) => {
                cfg.default_model = model.to_string();
            }
            #[cfg(test)]
            ProviderConfig::Mock => {}
        }
    } else {
        // No provider_config yet - we'll set the legacy field as a fallback
        // This will be used when provider is created later
        config.model = Some(model.to_string());
    }
}

fn get_default_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".miniclaw").join("config.json"))
}

fn merge_config_from_file(config: Config, path: &PathBuf) -> Result<Config> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(config),
        Err(e) => return Err(e).context("Failed to read metadata for config file"),
    };

    let permissions = metadata.permissions();
    let mode = permissions.mode() & 0o777;

    if mode != 0o600 {
        tracing::error!(
            "Config file {:?} has permissions {:o}, expected 0600 - skipping for security",
            path,
            mode
        );
        return Ok(config);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let file_config: Config = serde_json::from_str(&content).map_err(|e| {
        eprintln!("Error: Configuration file contains invalid JSON.");
        eprintln!("Suggestion: Run 'miniclaw onboard' to recreate the configuration file.");
        ConfigError::InvalidJson(e)
    })?;

    // Warn about deprecated 'model' field at root level
    if file_config.model.is_some() {
        tracing::warn!(
            "Config file contains deprecated 'model' field at root level. \
             This field is ignored. Use 'provider_config.default_model' instead. \
             Run 'miniclaw onboard' to update your configuration."
        );
    }

    // Warn about deprecated 'api_key' field at root level
    if file_config.api_key.is_some() {
        tracing::warn!(
            "Config file contains deprecated 'api_key' field at root level. \
             This field is ignored. Use 'provider_config.api_key' instead. \
             Run 'miniclaw onboard' to update your configuration."
        );
    }

    // Warn about deprecated 'provider_type' field at root level
    if file_config.provider_type.is_some() {
        tracing::warn!(
            "Config file contains deprecated 'provider_type' field at root level. \
             This field is ignored. Use 'provider_config.type' instead. \
             Run 'miniclaw onboard' to update your configuration."
        );
    }

    Ok(Config {
        api_key: None, // Deprecated, ignored
        model: None,   // Deprecated, ignored
        telegram_token: file_config.telegram_token.or(config.telegram_token),
        allow_from: if file_config.allow_from.is_empty() {
            config.allow_from
        } else {
            file_config.allow_from
        },
        spawn_log_output: file_config.spawn_log_output,
        default_channel: file_config.default_channel,
        provider_type: None, // Deprecated, ignored
        provider_config: file_config.provider_config.or(config.provider_config),
    })
}

fn merge_env_variables(config: Config) -> Config {
    // Check for provider-specific API keys and create provider_config
    // Filter out empty strings to avoid creating providers with empty keys
    let env_provider_config = if let Some(key) = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
    {
        Some(crate::providers::ProviderConfig::openai(key))
    } else if let Some(key) = std::env::var("KIMI_API_KEY").ok().filter(|k| !k.is_empty()) {
        Some(crate::providers::ProviderConfig::kimi(key))
    } else if let Some(key) = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .or(std::env::var("MINICLAW_API_KEY").ok())
        .filter(|k| !k.is_empty())
    {
        Some(crate::providers::ProviderConfig::openrouter(key))
    } else {
        None
    };

    Config {
        api_key: None, // Deprecated, ignored
        model: None,   // Deprecated, MINICLAW_MODEL env var removed
        telegram_token: std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .or(config.telegram_token),
        allow_from: std::env::var("MINICLAW_ALLOW_FROM")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .filter(|&x| x > 0)
                    .collect()
            })
            .filter(|v: &Vec<i64>| !v.is_empty())
            .unwrap_or(config.allow_from),
        spawn_log_output: config.spawn_log_output,
        default_channel: config.default_channel,
        provider_type: None, // Deprecated, ignored
        provider_config: env_provider_config.or(config.provider_config),
    }
}

pub fn save_config(config: &Config, path: &PathBuf) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
    }

    let json = serde_json::to_string_pretty(config)?;

    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create config file: {:?}", path))?;

    file.write_all(json.as_bytes())
        .with_context(|| format!("Failed to write config file: {:?}", path))?;

    // Set file permissions to 0600 (owner read/write only)
    let mut permissions = file.metadata()?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("Failed to set permissions on config file: {:?}", path))?;

    tracing::info!("Configuration saved to {:?}", path);
    Ok(())
}

pub fn get_config_path() -> Option<PathBuf> {
    get_default_config_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_load_config_defaults() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let nonexistent_path = temp_dir.path().join("nonexistent_config.json");

        // Clear any environment variables that could interfere
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("KIMI_API_KEY");
            env::remove_var("TELEGRAM_BOT_TOKEN");
            env::remove_var("MINICLAW_ALLOW_FROM");
        }

        // Use a non-existent config path to test pure defaults
        let config = load_config(None, Some(nonexistent_path)).unwrap();
        assert!(config.api_key.is_none());
        assert!(config.model.is_none());
        assert!(config.provider_config.is_none());
        // Note: telegram_token and allow_from might be set from environment, so we don't assert on them
    }

    #[test]
    fn test_load_config_from_file() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        // Clear any environment variables that could interfere
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("TELEGRAM_BOT_TOKEN");
            env::remove_var("MINICLAW_ALLOW_FROM");
        }

        // Create config with provider_config (new format)
        let test_config = Config {
            api_key: None, // Deprecated, should be ignored
            model: None,   // Deprecated, should be ignored
            telegram_token: Some("file-token".to_string()),
            allow_from: vec![123_456_789, 987_654_321],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None, // Deprecated, should be ignored
            provider_config: Some(crate::providers::ProviderConfig::openrouter("file-api-key")),
        };

        save_config(&test_config, &config_path).unwrap();

        let loaded = load_config(None, Some(config_path)).unwrap();
        // Deprecated fields should be None/ignored
        assert!(loaded.api_key.is_none());
        assert!(loaded.model.is_none());
        assert!(loaded.provider_type.is_none());
        // New format fields should be preserved
        assert!(loaded.provider_config.is_some());
        assert_eq!(loaded.telegram_token, Some("file-token".to_string()));
        assert_eq!(loaded.allow_from, vec![123_456_789, 987_654_321]);
    }

    #[test]
    fn test_load_config_invalid_json() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        fs::write(&config_path, "not valid json").unwrap();
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config_path, perms).unwrap();

        let result = load_config(None, Some(config_path));
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("invalid JSON") || err_msg.to_lowercase().contains("json"));
    }

    #[test]
    fn test_env_variable_override() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        // Create config file with provider_config
        let file_config = Config {
            api_key: None,
            model: None,
            telegram_token: Some("file-token".to_string()),
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(crate::providers::ProviderConfig::openai("file-key")),
        };
        save_config(&file_config, &config_path).unwrap();

        // Set environment variables - should override provider_config
        unsafe {
            env::set_var("OPENROUTER_API_KEY", "env-api-key");
            env::set_var("TELEGRAM_BOT_TOKEN", "env-token");
        }

        let config = load_config(None, Some(config_path)).unwrap();

        // Env provider_config should override file provider_config
        assert!(config.provider_config.is_some());
        assert_eq!(config.telegram_token, Some("env-token".to_string()));

        // Cleanup
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }

    #[test]
    fn test_cli_flag_override() {
        use crate::providers::{OpenAiConfig, ProviderConfig};

        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("TELEGRAM_BOT_TOKEN");
            env::remove_var("MINICLAW_ALLOW_FROM");
        }

        // Create config file with provider_config
        let file_config = Config {
            api_key: None,
            model: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(ProviderConfig::OpenAi(OpenAiConfig::new("file-key"))),
        };
        save_config(&file_config, &config_path).unwrap();

        // CLI --model flag should override provider_config.default_model
        let config = load_config(Some("cli-model".to_string()), Some(config_path)).unwrap();

        assert!(config.provider_config.is_some());
        assert_eq!(config.provider_config.unwrap().default_model(), "cli-model");

        // Cleanup - nothing to clean up
    }

    #[test]
    fn test_config_hierarchy_precedence() {
        use crate::providers::{OpenAiConfig, ProviderConfig};

        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("TELEGRAM_BOT_TOKEN");
            env::remove_var("MINICLAW_ALLOW_FROM");
        }

        // Layer 1: File config with provider_config
        let file_config = Config {
            api_key: None,
            model: None,
            telegram_token: Some("file-token".to_string()),
            allow_from: vec![111_111_111],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(ProviderConfig::OpenAi(OpenAiConfig::new("file-key"))),
        };
        save_config(&file_config, &config_path).unwrap();

        // Layer 2: Env overrides provider_config and allow_from
        unsafe {
            env::set_var("OPENROUTER_API_KEY", "env-api-key");
            env::set_var("MINICLAW_ALLOW_FROM", "222222222,333333333");
        }

        // Layer 3: CLI --model overrides provider_config.default_model
        let config = load_config(Some("cli-model".to_string()), Some(config_path)).unwrap();

        // Verify hierarchy: CLI > Env > File > Defaults
        assert!(config.provider_config.is_some());
        // CLI model override should change the model in provider_config
        assert_eq!(
            config.provider_config.as_ref().unwrap().default_model(),
            "cli-model"
        );
        assert_eq!(config.telegram_token, Some("file-token".to_string())); // From file
        assert_eq!(config.allow_from, vec![222_222_222, 333_333_333]); // From env

        // Cleanup
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_ALLOW_FROM");
        }
    }

    #[test]
    fn test_save_config_permissions() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        let config = Config::default();
        save_config(&config, &config_path).unwrap();

        let metadata = fs::metadata(&config_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode() & 0o777;

        assert_eq!(mode, 0o600, "Config file should have 0600 permissions");
    }

    #[test]
    fn test_allow_from_env() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var("MINICLAW_ALLOW_FROM", "123456789, 987654321, 111222333");
        }

        let config = load_config(None, None).unwrap();

        assert_eq!(
            config.allow_from,
            vec![123_456_789, 987_654_321, 111_222_333]
        );

        unsafe {
            env::remove_var("MINICLAW_ALLOW_FROM");
        }
    }

    #[test]
    fn test_allow_from_env_invalid_values_filtered() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var("MINICLAW_ALLOW_FROM", "123, abc, 0, -5, 456");
        }

        let config = load_config(None, None).unwrap();

        // Only valid positive integers should be kept
        assert_eq!(config.allow_from, vec![123, 456]);

        unsafe {
            env::remove_var("MINICLAW_ALLOW_FROM");
        }
    }

    #[test]
    fn test_get_config_path() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let path = get_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(".miniclaw"));
        assert!(path.to_string_lossy().contains("config.json"));
    }
}
