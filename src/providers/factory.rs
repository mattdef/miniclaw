//! Provider factory for creating LLM provider instances
//!
//! This module provides a factory pattern for creating provider instances based on configuration.
//! It supports multiple provider types and handles configuration validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::providers::{BoxedProvider, ProviderError};

/// Configuration for OpenRouter provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenRouterConfig {
    /// API key for OpenRouter
    pub api_key: String,
    /// Base URL for OpenRouter API (optional, defaults to official endpoint)
    #[serde(default = "default_openrouter_base_url")]
    pub base_url: String,
    /// Default model to use
    #[serde(default = "default_openrouter_model")]
    pub default_model: String,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    /// HTTP timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_openrouter_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_openrouter_model() -> String {
    "anthropic/claude-3.5-sonnet".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl OpenRouterConfig {
    /// Creates a new OpenRouter configuration with the required API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: default_openrouter_base_url(),
            default_model: default_openrouter_model(),
            organization_id: None,
            timeout_seconds: default_timeout(),
        }
    }

    /// Sets a custom base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the default model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Sets the organization ID
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Sets the timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.api_key.is_empty() {
            return Err(ProviderError::config("OpenRouter API key is required"));
        }

        if self.base_url.is_empty() {
            return Err(ProviderError::config("OpenRouter base URL cannot be empty"));
        }

        if self.default_model.is_empty() {
            return Err(ProviderError::config(
                "OpenRouter default model cannot be empty",
            ));
        }

        Ok(())
    }
}

/// Configuration for Ollama provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OllamaConfig {
    /// Base URL for Ollama API
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,
    /// Default model to use
    #[serde(default = "default_ollama_model")]
    pub default_model: String,
    /// HTTP timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Additional options for Ollama
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama3.2".to_string()
}

impl OllamaConfig {
    /// Creates a new Ollama configuration with default settings
    pub fn new() -> Self {
        Self {
            base_url: default_ollama_base_url(),
            default_model: default_ollama_model(),
            timeout_seconds: default_timeout(),
            options: HashMap::new(),
        }
    }

    /// Sets a custom base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Sets the default model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Sets the timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Adds an option
    pub fn with_option(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.base_url.is_empty() {
            return Err(ProviderError::config("Ollama base URL cannot be empty"));
        }

        if self.default_model.is_empty() {
            return Err(ProviderError::config(
                "Ollama default model cannot be empty",
            ));
        }

        Ok(())
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider configuration variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    /// OpenRouter provider configuration
    OpenRouter(OpenRouterConfig),
    /// Ollama local provider configuration
    Ollama(OllamaConfig),
    /// Mock provider for testing (only available in test builds)
    #[cfg(test)]
    Mock,
}

impl ProviderConfig {
    /// Returns the provider type name
    pub fn provider_type(&self) -> &'static str {
        match self {
            ProviderConfig::OpenRouter(_) => "openrouter",
            ProviderConfig::Ollama(_) => "ollama",
            #[cfg(test)]
            ProviderConfig::Mock => "mock",
        }
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), ProviderError> {
        match self {
            ProviderConfig::OpenRouter(config) => config.validate(),
            ProviderConfig::Ollama(config) => config.validate(),
            #[cfg(test)]
            ProviderConfig::Mock => Ok(()),
        }
    }

    /// Creates an OpenRouter configuration
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::OpenRouter(OpenRouterConfig::new(api_key))
    }

    /// Creates an Ollama configuration
    pub fn ollama() -> Self {
        Self::Ollama(OllamaConfig::new())
    }

    #[cfg(test)]
    /// Creates a mock configuration
    pub fn mock() -> Self {
        Self::Mock
    }
}

/// Factory for creating LLM provider instances
///
/// The factory pattern allows runtime selection of providers based on configuration.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Creates a provider instance from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Provider configuration
    ///
    /// # Returns
    ///
    /// A boxed provider instance or an error if configuration is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use miniclaw::providers::{ProviderConfig, ProviderFactory};
    ///
    /// async fn example() {
    ///     let config = ProviderConfig::openrouter("api-key");
    ///     let provider = ProviderFactory::create(config).unwrap();
    ///     // Use provider...
    /// }
    /// ```
    pub fn create(config: ProviderConfig) -> Result<BoxedProvider, ProviderError> {
        // Validate configuration before creating provider
        config.validate()?;

        match config {
            ProviderConfig::OpenRouter(_config) => {
                // TODO: Create OpenRouter provider when implemented in Story 4.2
                Err(ProviderError::config(
                    "OpenRouter provider not yet implemented. See Story 4.2",
                ))
            }
            ProviderConfig::Ollama(_config) => {
                // TODO: Create Ollama provider when implemented in Story 4.3
                Err(ProviderError::config(
                    "Ollama provider not yet implemented. See Story 4.3",
                ))
            }
            #[cfg(test)]
            ProviderConfig::Mock => {
                // Mock provider will be created in tests
                Err(ProviderError::config(
                    "Mock provider should be created directly in tests",
                ))
            }
        }
    }

    /// Creates a provider from JSON configuration
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string containing provider configuration
    ///
    /// # Returns
    ///
    /// A boxed provider instance or an error
    pub fn create_from_json(json: &str) -> Result<BoxedProvider, ProviderError> {
        let config: ProviderConfig =
            serde_json::from_str(json).map_err(|e| ProviderError::config(e.to_string()))?;
        Self::create(config)
    }

    /// Lists available provider types
    pub fn available_providers() -> Vec<&'static str> {
        vec!["openrouter", "ollama"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_config_validation() {
        // Valid config
        let config = OpenRouterConfig::new("valid-key");
        assert!(config.validate().is_ok());

        // Invalid - empty API key
        let config = OpenRouterConfig {
            api_key: "".to_string(),
            ..OpenRouterConfig::new("placeholder")
        };
        assert!(config.validate().is_err());

        // Invalid - empty base URL
        let config = OpenRouterConfig {
            base_url: "".to_string(),
            ..OpenRouterConfig::new("key")
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_openrouter_config_builder() {
        let config = OpenRouterConfig::new("my-key")
            .with_model("custom-model")
            .with_timeout(60)
            .with_organization("my-org");

        assert_eq!(config.api_key, "my-key");
        assert_eq!(config.default_model, "custom-model");
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.organization_id, Some("my-org".to_string()));
    }

    #[test]
    fn test_ollama_config_validation() {
        // Valid config
        let config = OllamaConfig::new();
        assert!(config.validate().is_ok());

        // Invalid - empty base URL
        let config = OllamaConfig {
            base_url: "".to_string(),
            ..OllamaConfig::new()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ollama_config_builder() {
        let config = OllamaConfig::new()
            .with_model("mistral")
            .with_base_url("http://custom:11434")
            .with_option("temperature", 0.7);

        assert_eq!(config.default_model, "mistral");
        assert_eq!(config.base_url, "http://custom:11434");
        assert_eq!(
            config.options.get("temperature"),
            Some(&serde_json::json!(0.7))
        );
    }

    #[test]
    fn test_provider_config_variants() {
        let openrouter = ProviderConfig::openrouter("key");
        assert_eq!(openrouter.provider_type(), "openrouter");
        assert!(matches!(openrouter, ProviderConfig::OpenRouter(_)));

        let ollama = ProviderConfig::ollama();
        assert_eq!(ollama.provider_type(), "ollama");
        assert!(matches!(ollama, ProviderConfig::Ollama(_)));
    }

    #[test]
    fn test_provider_config_serialization() {
        let config = ProviderConfig::openrouter("test-key");
        let json = serde_json::to_string(&config).unwrap();

        // Should be valid JSON and deserializable
        let decoded: ProviderConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, ProviderConfig::OpenRouter(_)));

        // Verify it contains the API key
        assert!(json.contains("test-key"));
    }

    #[test]
    fn test_factory_create_openrouter_returns_error() {
        let config = ProviderConfig::openrouter("test-key");
        let result = ProviderFactory::create(config);

        // Should fail since OpenRouter provider not implemented yet
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("not yet implemented"));
        }
    }

    #[test]
    fn test_factory_create_ollama_returns_error() {
        let config = ProviderConfig::ollama();
        let result = ProviderFactory::create(config);

        // Should fail since Ollama provider not implemented yet
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("not yet implemented"));
        }
    }

    #[test]
    fn test_factory_create_from_json() {
        let json = r#"{"type":"openrouter","api_key":"test","default_model":"model"}"#;
        let result = ProviderFactory::create_from_json(json);

        // Should fail since provider not implemented yet
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_create_from_invalid_json() {
        let json = r#"invalid json"#;
        let result = ProviderFactory::create_from_json(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_available_providers() {
        let providers = ProviderFactory::available_providers();
        assert!(providers.contains(&"openrouter"));
        assert!(providers.contains(&"ollama"));
    }

    #[test]
    fn test_provider_config_default_values() {
        let config = OpenRouterConfig::new("key");
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.default_model, "anthropic/claude-3.5-sonnet");
        assert_eq!(config.timeout_seconds, 30);

        let config = OllamaConfig::new();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.default_model, "llama3.2");
        assert_eq!(config.timeout_seconds, 30);
    }
}
