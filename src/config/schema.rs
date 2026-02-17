use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_token: Option<String>,

    /// Telegram user IDs allowed to interact with the bot (NFR-S5)
    /// Empty vector = secure by default (no one allowed)
    #[serde(default)]
    pub allow_from: Vec<i64>,

    /// Whether to log stdout/stderr output from spawned processes
    #[serde(default = "default_spawn_log_output")]
    pub spawn_log_output: bool,
}

fn default_spawn_log_output() -> bool {
    false
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            model: Some("google/gemini-2.5-flash".to_string()),
            telegram_token: None,
            allow_from: Vec::new(),
            spawn_log_output: false,
        }
    }
}

impl Config {
    /// Validate the configuration.
    ///
    /// Checks:
    /// - All user IDs in allow_from are positive integers
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate allow_from entries are positive integers
        for user_id in &self.allow_from {
            if *user_id <= 0 {
                return Err(anyhow::anyhow!(
                    "Invalid user ID in allow_from: {}. User IDs must be positive integers.",
                    user_id
                ));
            }
        }

        Ok(())
    }

    /// Check if OpenAI-compatible API key is configured (for logging purposes).
    /// Returns true if set, without exposing the actual key value.
    pub fn is_api_key_configured(&self) -> bool {
        self.api_key.as_ref().is_some_and(|k| !k.is_empty())
    }

    /// Check if Telegram token is configured (for logging purposes).
    /// Returns true if set, without exposing the actual token value.
    pub fn is_telegram_configured(&self) -> bool {
        self.telegram_token.as_ref().is_some_and(|t| !t.is_empty())
    }

    /// Check if model is explicitly configured.
    pub fn is_model_configured(&self) -> bool {
        self.model.as_ref().is_some_and(|m| !m.is_empty())
    }

    /// Get safe configuration summary for logging.
    /// Never includes actual secret values.
    pub fn get_safe_summary(&self) -> SafeConfigSummary {
        SafeConfigSummary {
            api_key_configured: self.is_api_key_configured(),
            telegram_configured: self.is_telegram_configured(),
            model_configured: self.is_model_configured(),
            model: self.model.clone(),
            allow_from_count: self.allow_from.len(),
            spawn_log_output: self.spawn_log_output,
        }
    }
}

/// Safe configuration summary for logging.
/// Contains no sensitive data - only boolean flags and safe metadata.
#[derive(Debug)]
pub struct SafeConfigSummary {
    pub api_key_configured: bool,
    pub telegram_configured: bool,
    pub model_configured: bool,
    pub model: Option<String>,
    pub allow_from_count: usize,
    pub spawn_log_output: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.api_key.is_none());
        assert_eq!(config.model, Some("google/gemini-2.5-flash".to_string()));
        assert!(config.telegram_token.is_none());
        assert!(config.allow_from.is_empty()); // Secure by default
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            api_key: Some("test-key".to_string()),
            model: Some("test-model".to_string()),
            telegram_token: None,
            allow_from: vec![123_456_789, 987_654_321],
            spawn_log_output: false,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("test-model"));
        assert!(json.contains("123456789"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "api_key": "my-api-key",
            "model": "custom-model",
            "telegram_token": "bot-token",
            "allow_from": [123456789]
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.model, Some("custom-model".to_string()));
        assert_eq!(config.telegram_token, Some("bot-token".to_string()));
        assert_eq!(config.allow_from, vec![123_456_789]);
    }

    #[test]
    fn test_config_deserialization_without_allow_from() {
        let json = r#"{
            "api_key": "my-api-key",
            "telegram_token": "bot-token"
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.allow_from.is_empty()); // Default to empty
    }

    #[test]
    fn test_config_validate_accepts_valid_user_ids() {
        let config = Config {
            api_key: None,
            model: None,
            telegram_token: None,
            allow_from: vec![123_456_789, 1, i64::MAX],
            spawn_log_output: false,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_rejects_invalid_user_id_zero() {
        let config = Config {
            api_key: None,
            model: None,
            telegram_token: None,
            allow_from: vec![0],
            spawn_log_output: false,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_invalid_user_id_negative() {
        let config = Config {
            api_key: None,
            model: None,
            telegram_token: None,
            allow_from: vec![-1, -123_456],
            spawn_log_output: false,
        };

        assert!(config.validate().is_err());
    }
}
