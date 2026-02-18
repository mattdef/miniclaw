use serde::{Deserialize, Serialize};

use crate::providers::ProviderConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_token: Option<String>,

    /// Telegram user IDs allowed to interact with the bot (NFR-S5)
    /// Empty vector = secure by default (no one allowed)
    #[serde(default)]
    pub allow_from: Vec<i64>,

    /// Whether to log stdout/stderr output from spawned processes
    #[serde(default = "default_spawn_log_output")]
    pub spawn_log_output: bool,

    /// Default channel for message tool (e.g., "telegram", "cli")
    #[serde(default = "default_channel")]
    pub default_channel: String,

    /// Provider type: "openrouter", "openai", "kimi", or "ollama"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,

    /// Full provider configuration (replaces api_key for new configs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_config: Option<ProviderConfig>,

    /// DEPRECATED: Legacy model field - kept for deserialization warning only
    #[serde(skip_serializing)]
    pub model: Option<String>,
}

fn default_spawn_log_output() -> bool {
    false
}

fn default_channel() -> String {
    "telegram".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            telegram_token: None,
            allow_from: Vec::new(),
            spawn_log_output: false,
            default_channel: default_channel(),
            provider_type: None,
            provider_config: None,
            model: None,
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

    /// Get safe configuration summary for logging.
    /// Never includes actual secret values.
    pub fn get_safe_summary(&self) -> SafeConfigSummary {
        SafeConfigSummary {
            api_key_configured: self.is_api_key_configured(),
            telegram_configured: self.is_telegram_configured(),
            allow_from_count: self.allow_from.len(),
            spawn_log_output: self.spawn_log_output,
            provider_type: self.provider_type.clone(),
            provider_configured: self.provider_config.is_some(),
            model: self
                .provider_config
                .as_ref()
                .map(|pc| pc.default_model().to_string()),
        }
    }
}

/// Safe configuration summary for logging.
/// Contains no sensitive data - only boolean flags and safe metadata.
#[derive(Debug)]
pub struct SafeConfigSummary {
    pub api_key_configured: bool,
    pub telegram_configured: bool,
    pub allow_from_count: usize,
    pub spawn_log_output: bool,
    pub provider_type: Option<String>,
    pub provider_configured: bool,
    pub model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.api_key.is_none());
        assert!(config.model.is_none());
        assert!(config.telegram_token.is_none());
        assert!(config.allow_from.is_empty()); // Secure by default
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            api_key: Some("test-key".to_string()),
            telegram_token: None,
            allow_from: vec![123_456_789, 987_654_321],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("123456789"));
        // model should NOT be serialized (deprecated field)
        assert!(!json.contains("model"));
    }

    #[test]
    fn test_config_deserialization_with_deprecated_model() {
        // Test that old configs with "model" field can still be deserialized
        let json = r#"{
            "api_key": "my-api-key",
            "model": "custom-model",
            "telegram_token": "bot-token",
            "allow_from": [123456789]
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.model, Some("custom-model".to_string())); // Still parsed but ignored
        assert_eq!(config.telegram_token, Some("bot-token".to_string()));
        assert_eq!(config.allow_from, vec![123_456_789]);
    }

    #[test]
    fn test_config_deserialization_without_model() {
        let json = r#"{
            "api_key": "my-api-key",
            "telegram_token": "bot-token",
            "allow_from": [123456789]
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert!(config.model.is_none());
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
        assert!(config.model.is_none());
    }

    #[test]
    fn test_config_validate_accepts_valid_user_ids() {
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![123_456_789, 1, i64::MAX],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_rejects_invalid_user_id_zero() {
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![0],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_rejects_invalid_user_id_negative() {
        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![-1, -123_456],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: None,
            model: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_safe_summary_with_provider_config() {
        use crate::providers::{OpenAiConfig, ProviderConfig};

        let config = Config {
            api_key: None,
            telegram_token: None,
            allow_from: vec![],
            spawn_log_output: false,
            default_channel: "telegram".to_string(),
            provider_type: None,
            provider_config: Some(ProviderConfig::OpenAi(OpenAiConfig::new("test-key"))),
            model: None,
        };

        let summary = config.get_safe_summary();
        assert_eq!(summary.model, Some("gpt-4o".to_string()));
        assert!(summary.provider_configured);
    }

    #[test]
    fn test_safe_summary_without_provider_config() {
        let config = Config::default();
        let summary = config.get_safe_summary();
        assert!(summary.model.is_none());
        assert!(!summary.provider_configured);
    }
}
