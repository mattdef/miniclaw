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
    let mut config = Config::default();

    // Layer 1: Load from config file (~/.miniclaw/config.json)
    let config_file = cli_config_path.or_else(get_default_config_path);

    if let Some(path) = config_file {
        if path.exists() {
            config = merge_config_from_file(config, &path)?;
        }
    }

    // Layer 2: Environment variables override
    config = merge_env_variables(config);

    // Layer 3: CLI flags override (highest precedence)
    if let Some(model) = cli_model {
        config.model = Some(model);
    }

    Ok(config)
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
        tracing::warn!(
            "Config file {:?} has permissions {:o}, expected 0600 - skipping",
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

    Ok(Config {
        api_key: file_config.api_key.or(config.api_key),
        model: file_config.model.or(config.model),
        telegram_token: file_config.telegram_token.or(config.telegram_token),
        telegram_whitelist: file_config.telegram_whitelist.or(config.telegram_whitelist),
    })
}

fn merge_env_variables(config: Config) -> Config {
    Config {
        api_key: std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or(std::env::var("MINICLAW_API_KEY").ok())
            .or_else(|| config.api_key),
        model: std::env::var("MINICLAW_MODEL")
            .ok()
            .or_else(|| config.model),
        telegram_token: std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .or_else(|| config.telegram_token),
        telegram_whitelist: std::env::var("MINICLAW_TELEGRAM_WHITELIST")
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .or_else(|| config.telegram_whitelist),
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
        // Clear any environment variables that could interfere
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("MINICLAW_MODEL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
            env::remove_var("MINICLAW_TELEGRAM_WHITELIST");
        }

        let config = load_config(None, None).unwrap();
        assert!(config.api_key.is_none());
        assert_eq!(config.model, Some("google/gemini-2.5-flash".to_string()));
        assert!(config.telegram_token.is_none());
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
            env::remove_var("MINICLAW_MODEL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }

        let test_config = Config {
            api_key: Some("file-api-key".to_string()),
            model: Some("file-model".to_string()),
            telegram_token: Some("file-token".to_string()),
            telegram_whitelist: Some(vec!["user1".to_string()]),
        };

        save_config(&test_config, &config_path).unwrap();

        let loaded = load_config(None, Some(config_path)).unwrap();
        assert_eq!(loaded.api_key, Some("file-api-key".to_string()));
        assert_eq!(loaded.model, Some("file-model".to_string()));
        assert_eq!(loaded.telegram_token, Some("file-token".to_string()));
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

        // Create config file
        let file_config = Config {
            api_key: Some("file-key".to_string()),
            model: Some("file-model".to_string()),
            telegram_token: Some("file-token".to_string()),
            telegram_whitelist: None,
        };
        save_config(&file_config, &config_path).unwrap();

        // Set environment variables
        unsafe {
            env::set_var("OPENROUTER_API_KEY", "env-api-key");
            env::set_var("MINICLAW_MODEL", "env-model");
            env::set_var("TELEGRAM_BOT_TOKEN", "env-token");
        }

        let config = load_config(None, Some(config_path)).unwrap();

        assert_eq!(config.api_key, Some("env-api-key".to_string()));
        assert_eq!(config.model, Some("env-model".to_string()));
        assert_eq!(config.telegram_token, Some("env-token".to_string()));

        // Cleanup
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_MODEL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }

    #[test]
    fn test_cli_flag_override() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("MINICLAW_MODEL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }

        // Create config file
        let file_config = Config {
            api_key: Some("file-key".to_string()),
            model: Some("file-model".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        };
        save_config(&file_config, &config_path).unwrap();

        // Set environment variable
        unsafe {
            env::set_var("MINICLAW_MODEL", "env-model");
        }

        // CLI flag should override env
        let config = load_config(Some("cli-model".to_string()), Some(config_path)).unwrap();

        assert_eq!(config.api_key, Some("file-key".to_string()));
        assert_eq!(config.model, Some("cli-model".to_string()));

        // Cleanup
        unsafe {
            env::remove_var("MINICLAW_MODEL");
        }
    }

    #[test]
    fn test_config_hierarchy_precedence() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        let temp_dir = setup();
        let config_path = temp_dir.path().join("config.json");

        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
            env::remove_var("MINICLAW_API_KEY");
            env::remove_var("MINICLAW_MODEL");
            env::remove_var("TELEGRAM_BOT_TOKEN");
        }

        // Layer 1: File config with api_key
        let file_config = Config {
            api_key: Some("file-api-key".to_string()),
            model: Some("file-model".to_string()),
            telegram_token: Some("file-token".to_string()),
            telegram_whitelist: None,
        };
        save_config(&file_config, &config_path).unwrap();

        // Layer 2: Env overrides api_key
        unsafe {
            env::set_var("OPENROUTER_API_KEY", "env-api-key");
        }

        // Layer 3: CLI overrides model
        let config = load_config(Some("cli-model".to_string()), Some(config_path)).unwrap();

        // Verify hierarchy: CLI > Env > File > Defaults
        assert_eq!(config.api_key, Some("env-api-key".to_string())); // From env
        assert_eq!(config.model, Some("cli-model".to_string())); // From CLI
        assert_eq!(config.telegram_token, Some("file-token".to_string())); // From file

        // Cleanup
        unsafe {
            env::remove_var("OPENROUTER_API_KEY");
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
    fn test_telegram_whitelist_env() {
        let _lock = CONFIG_TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var("MINICLAW_TELEGRAM_WHITELIST", "user1, user2, user3");
        }

        let config = load_config(None, None).unwrap();

        assert_eq!(
            config.telegram_whitelist,
            Some(vec![
                "user1".to_string(),
                "user2".to_string(),
                "user3".to_string()
            ])
        );

        unsafe {
            env::remove_var("MINICLAW_TELEGRAM_WHITELIST");
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
