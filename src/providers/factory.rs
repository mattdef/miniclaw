//! Provider factory for creating LLM provider instances
//!
//! This module provides a factory pattern for creating provider instances based on configuration.
//! It supports multiple provider types and handles configuration validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::providers::ollama::OllamaProvider;
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

/// Configuration for OpenAI provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenAiConfig {
    /// API key for OpenAI
    pub api_key: String,
    /// Base URL for OpenAI API (optional, defaults to official endpoint)
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,
    /// Default model to use
    #[serde(default = "default_openai_model")]
    pub default_model: String,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    /// HTTP timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_model() -> String {
    "gpt-4o".to_string()
}

impl OpenAiConfig {
    /// Creates a new OpenAI configuration with the required API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: default_openai_base_url(),
            default_model: default_openai_model(),
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
            return Err(ProviderError::config("OpenAI API key is required"));
        }

        if self.base_url.is_empty() {
            return Err(ProviderError::config("OpenAI base URL cannot be empty"));
        }

        if self.default_model.is_empty() {
            return Err(ProviderError::config(
                "OpenAI default model cannot be empty",
            ));
        }

        Ok(())
    }
}

/// Configuration for Kimi (Moonshot AI) provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KimiConfig {
    /// API key for Kimi
    pub api_key: String,
    /// Base URL for Kimi API (optional, defaults to official endpoint)
    #[serde(default = "default_kimi_base_url")]
    pub base_url: String,
    /// Default model to use
    #[serde(default = "default_kimi_model")]
    pub default_model: String,
    /// HTTP timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_kimi_base_url() -> String {
    "https://api.moonshot.cn/v1".to_string()
}

fn default_kimi_model() -> String {
    "kimi-latest".to_string()
}

impl KimiConfig {
    /// Creates a new Kimi configuration with the required API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: default_kimi_base_url(),
            default_model: default_kimi_model(),
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

    /// Sets the timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.api_key.is_empty() {
            return Err(ProviderError::config("Kimi API key is required"));
        }

        if self.base_url.is_empty() {
            return Err(ProviderError::config("Kimi base URL cannot be empty"));
        }

        if self.default_model.is_empty() {
            return Err(ProviderError::config("Kimi default model cannot be empty"));
        }

        Ok(())
    }
}

/// Trait for provider configurations that require an API key.
///
/// Implemented by [`OpenRouterConfig`], [`OpenAiConfig`], and [`KimiConfig`].
/// Not implemented for [`OllamaConfig`] (local provider, no key required).
pub trait ApiKeyProviderConfig {
    /// Returns the API key for this provider
    fn api_key(&self) -> &str;

    /// Sets the API key for this provider
    fn set_api_key(&mut self, key: String);
}

impl ApiKeyProviderConfig for OpenRouterConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn set_api_key(&mut self, key: String) {
        self.api_key = key;
    }
}

impl ApiKeyProviderConfig for OpenAiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn set_api_key(&mut self, key: String) {
        self.api_key = key;
    }
}

impl ApiKeyProviderConfig for KimiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn set_api_key(&mut self, key: String) {
        self.api_key = key;
    }
}

/// Provider configuration variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    /// OpenRouter provider configuration
    #[serde(rename = "openrouter")]
    OpenRouter(OpenRouterConfig),
    /// OpenAI native provider configuration
    #[serde(rename = "openai")]
    OpenAi(OpenAiConfig),
    /// Kimi (Moonshot AI) provider configuration
    #[serde(rename = "kimi")]
    Kimi(KimiConfig),
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
            ProviderConfig::OpenAi(_) => "openai",
            ProviderConfig::Kimi(_) => "kimi",
            ProviderConfig::Ollama(_) => "ollama",
            #[cfg(test)]
            ProviderConfig::Mock => "mock",
        }
    }

    /// Validates the configuration
    pub fn validate(&self) -> Result<(), ProviderError> {
        match self {
            ProviderConfig::OpenRouter(config) => config.validate(),
            ProviderConfig::OpenAi(config) => config.validate(),
            ProviderConfig::Kimi(config) => config.validate(),
            ProviderConfig::Ollama(config) => config.validate(),
            #[cfg(test)]
            ProviderConfig::Mock => Ok(()),
        }
    }

    /// Returns the default model for this provider configuration
    pub fn default_model(&self) -> &str {
        match self {
            ProviderConfig::OpenRouter(config) => &config.default_model,
            ProviderConfig::OpenAi(config) => &config.default_model,
            ProviderConfig::Kimi(config) => &config.default_model,
            ProviderConfig::Ollama(config) => &config.default_model,
            #[cfg(test)]
            ProviderConfig::Mock => "mock-model",
        }
    }

    /// Sets the default model for this provider configuration
    pub fn set_default_model(&mut self, model: String) {
        match self {
            ProviderConfig::OpenRouter(config) => config.default_model = model,
            ProviderConfig::OpenAi(config) => config.default_model = model,
            ProviderConfig::Kimi(config) => config.default_model = model,
            ProviderConfig::Ollama(config) => config.default_model = model,
            #[cfg(test)]
            ProviderConfig::Mock => {}
        }
    }

    /// Creates an OpenRouter configuration
    pub fn openrouter(api_key: impl Into<String>) -> Self {
        Self::OpenRouter(OpenRouterConfig::new(api_key))
    }

    /// Creates an OpenAI configuration
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::OpenAi(OpenAiConfig::new(api_key))
    }

    /// Creates a Kimi configuration
    pub fn kimi(api_key: impl Into<String>) -> Self {
        Self::Kimi(KimiConfig::new(api_key))
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
            ProviderConfig::OpenRouter(config) => {
                // Create OpenRouter provider with the given configuration
                use crate::providers::openai::GenericOpenAiProvider;
                let provider = GenericOpenAiProvider::from_openrouter_config(config);
                Ok(Box::new(provider))
            }
            ProviderConfig::OpenAi(config) => {
                // Create OpenAI native provider
                use crate::providers::openai::GenericOpenAiProvider;
                let provider = GenericOpenAiProvider::from_openai_config(config);
                Ok(Box::new(provider))
            }
            ProviderConfig::Kimi(config) => {
                // Create Kimi provider (Moonshot AI - OpenAI compatible)
                use crate::providers::openai::GenericOpenAiProvider;
                let provider = GenericOpenAiProvider::new(
                    config.api_key,
                    config.base_url,
                    config.default_model,
                    "kimi",
                    None,
                    config.timeout_seconds,
                );
                Ok(Box::new(provider))
            }
            ProviderConfig::Ollama(config) => {
                // Create Ollama provider with the given configuration using fallible constructor
                let provider = OllamaProvider::try_new(config)?;
                Ok(Box::new(provider))
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
        vec!["openrouter", "openai", "kimi", "ollama"]
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
    fn test_factory_create_openrouter_success() {
        let config = ProviderConfig::openrouter("test-key");
        let result = ProviderFactory::create(config);

        // Should succeed now that OpenRouter provider is implemented
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "openrouter");
        assert_eq!(provider.default_model(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_factory_create_ollama_success() {
        let config = ProviderConfig::ollama();
        let result = ProviderFactory::create(config);

        // Should succeed now that Ollama provider is implemented
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "ollama");
        assert_eq!(provider.default_model(), "llama3.2");
    }

    #[test]
    fn test_factory_create_from_json() {
        let json = r#"{"type":"openrouter","api_key":"test","default_model":"model"}"#;
        let result = ProviderFactory::create_from_json(json);

        // Debug: print the error if it fails
        match &result {
            Ok(_) => {}
            Err(e) => eprintln!("Error creating provider from JSON: {}", e),
        }

        // Should succeed now that OpenRouter provider is implemented
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.provider_name(), "openrouter");
        assert_eq!(provider.default_model(), "model");
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

    #[test]
    fn test_provider_config_default_model_method() {
        // Test default_model() method returns the correct model for each variant
        let openrouter = ProviderConfig::openrouter("key");
        assert_eq!(openrouter.default_model(), "anthropic/claude-3.5-sonnet");

        let openai = ProviderConfig::openai("key");
        assert_eq!(openai.default_model(), "gpt-4o");

        let kimi = ProviderConfig::kimi("key");
        assert_eq!(kimi.default_model(), "kimi-latest");

        let ollama = ProviderConfig::ollama();
        assert_eq!(ollama.default_model(), "llama3.2");

        // Test with custom models
        let openrouter_custom =
            ProviderConfig::OpenRouter(OpenRouterConfig::new("key").with_model("custom-model"));
        assert_eq!(openrouter_custom.default_model(), "custom-model");

        let openai_custom =
            ProviderConfig::OpenAi(OpenAiConfig::new("key").with_model("gpt-4-turbo"));
        assert_eq!(openai_custom.default_model(), "gpt-4-turbo");
    }
}
