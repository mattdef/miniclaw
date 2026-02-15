//! Error types for LLM provider operations
//!
//! This module defines the error types that can occur when interacting with LLM providers.
//! Errors are categorized to allow appropriate handling strategies (retry vs fail).

use thiserror::Error;

/// Errors that can occur when interacting with LLM providers
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ProviderError {
    /// Network-related errors (connection issues, timeouts, DNS failures)
    #[error("Network error: {message}")]
    Network {
        /// Error message
        message: String,
    },

    /// Authentication errors (invalid API key, unauthorized access)
    #[error("Authentication error: {message}")]
    Auth {
        /// Error message
        message: String,
    },

    /// Rate limit errors (too many requests, quota exceeded)
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        /// Error message
        message: String,
        /// Optional retry-after duration in seconds
        retry_after: Option<u64>,
    },

    /// Invalid request errors (bad parameters, malformed input)
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Error message
        message: String,
    },

    /// Timeout errors (request took too long)
    #[error("Request timeout after {seconds} seconds")]
    Timeout {
        /// Timeout duration in seconds
        seconds: u64,
    },

    /// Provider-specific errors (errors from the LLM service)
    #[error("Provider error: {message}")]
    Provider {
        /// Error message from provider
        message: String,
        /// Optional error code from provider
        code: Option<String>,
    },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization {
        /// Error message
        message: String,
    },

    /// Configuration errors (missing required config, invalid settings)
    #[error("Configuration error: {message}")]
    Config {
        /// Error message
        message: String,
    },

    /// Unknown/unexpected errors
    #[error("Unknown error: {message}")]
    Unknown {
        /// Error message
        message: String,
    },
}

impl ProviderError {
    /// Returns true if this error is potentially retryable
    ///
    /// Retryable errors are typically transient and may succeed on retry:
    /// - Network errors
    /// - Rate limit errors (after waiting)
    /// - Timeout errors
    /// - Some provider errors (server errors)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::Network { .. }
                | ProviderError::RateLimit { .. }
                | ProviderError::Timeout { .. }
        )
    }

    /// Returns true if this error indicates an authentication problem
    ///
    /// Auth errors are not retryable without user intervention
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ProviderError::Auth { .. })
    }

    /// Returns true if this error indicates a rate limit
    ///
    /// Rate limits can be retried after waiting
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ProviderError::RateLimit { .. })
    }

    /// Returns the suggested retry delay in seconds, if any
    ///
    /// For rate limits, this may be specified by the provider.
    /// For other retryable errors, returns a default value.
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimit { retry_after, .. } => *retry_after,
            ProviderError::Network { .. } => Some(1), // Default 1 second for network errors
            ProviderError::Timeout { .. } => Some(2), // Default 2 seconds for timeouts
            _ => None,
        }
    }

    /// Creates a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Creates an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// Creates a rate limit error
    pub fn rate_limit(message: impl Into<String>, retry_after: Option<u64>) -> Self {
        Self::RateLimit {
            message: message.into(),
            retry_after,
        }
    }

    /// Creates an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Creates a timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Creates a provider error
    pub fn provider(message: impl Into<String>, code: Option<impl Into<String>>) -> Self {
        Self::Provider {
            message: message.into(),
            code: code.map(|c| c.into()),
        }
    }

    /// Creates a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Creates a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Creates an unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::Unknown {
            message: message.into(),
        }
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}

// TODO: Add From<reqwest::Error> implementation when reqwest is added in Story 4.2
// impl From<reqwest::Error> for ProviderError {
//     fn from(err: reqwest::Error) -> Self {
//         if err.is_timeout() {
//             Self::Timeout { seconds: 30 }
//         } else if err.is_connect() {
//             Self::Network { message: err.to_string() }
//         } else {
//             Self::Network { message: err.to_string() }
//         }
//     }
// }

/// Result type for provider operations
pub type Result<T> = std::result::Result<T, ProviderError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error_is_retryable() {
        let err = ProviderError::network("Connection failed");
        assert!(err.is_retryable());
        assert!(!err.is_auth_error());
        assert_eq!(err.retry_after(), Some(1));
    }

    #[test]
    fn test_auth_error_not_retryable() {
        let err = ProviderError::auth("Invalid API key");
        assert!(!err.is_retryable());
        assert!(err.is_auth_error());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn test_rate_limit_error() {
        let err = ProviderError::rate_limit("Too many requests", Some(60));
        assert!(err.is_retryable());
        assert!(err.is_rate_limit());
        assert_eq!(err.retry_after(), Some(60));
    }

    #[test]
    fn test_timeout_error() {
        let err = ProviderError::timeout(30);
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(2));
    }

    #[test]
    fn test_invalid_request_not_retryable() {
        let err = ProviderError::invalid_request("Bad parameter");
        assert!(!err.is_retryable());
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_provider_error_with_code() {
        let err = ProviderError::provider("Server error", Some::<&str>("500"));
        assert!(!err.is_retryable());
        assert!(matches!(err, ProviderError::Provider { code: Some(_), .. }));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let provider_err: ProviderError = json_err.into();
        assert!(matches!(provider_err, ProviderError::Serialization { .. }));
    }

    #[test]
    fn test_error_display_messages() {
        let err = ProviderError::network("Connection refused");
        assert!(err.to_string().contains("Network error"));
        assert!(err.to_string().contains("Connection refused"));

        let err = ProviderError::auth("Invalid key");
        assert!(err.to_string().contains("Authentication error"));

        let err = ProviderError::rate_limit("Too many requests", None);
        assert!(err.to_string().contains("Rate limit exceeded"));
    }

    #[test]
    fn test_result_type() {
        fn may_fail() -> Result<i32> {
            Ok(42)
        }

        fn always_fails() -> Result<i32> {
            Err(ProviderError::unknown("test"))
        }

        assert_eq!(may_fail().unwrap(), 42);
        assert!(always_fails().is_err());
    }
}
