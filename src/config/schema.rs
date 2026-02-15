use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_whitelist: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: None,
            model: Some("google/gemini-2.5-flash".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        }
    }
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
        assert!(config.telegram_whitelist.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            api_key: Some("test-key".to_string()),
            model: Some("test-model".to_string()),
            telegram_token: None,
            telegram_whitelist: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("test-model"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "api_key": "my-api-key",
            "model": "custom-model",
            "telegram_token": "bot-token"
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.model, Some("custom-model".to_string()));
        assert_eq!(config.telegram_token, Some("bot-token".to_string()));
    }
}
