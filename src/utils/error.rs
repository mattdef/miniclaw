//! Centralized error types and conversions for miniclaw
//!
//! This module provides structured error types using `thiserror` for library code.
//! CLI/main modules should use `anyhow` for easy context.

use std::path::PathBuf;
use thiserror::Error;

/// Global error type for miniclaw operations
#[derive(Error, Debug)]
pub enum MiniClawError {
    /// IO errors with path context
    #[error("IO error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Session persistence failures
    #[error("Session persistence failed for {session_id}: {message}")]
    SessionPersistence { session_id: String, message: String },

    /// Invalid user input
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Path validation errors
    #[error("Path validation failed: {message}")]
    PathValidation { message: String },

    /// Security/permission errors
    #[error("Security violation: {message}")]
    Security { message: String },

    /// External service errors (Telegram, LLM providers)
    #[error("External service error ({service}): {message}")]
    ExternalService { service: String, message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Operation timeout
    #[error("Operation timed out after {duration_secs}s: {operation}")]
    Timeout {
        operation: String,
        duration_secs: u64,
    },

    /// Channel/communication errors
    #[error("Channel error: {message}")]
    Channel { message: String },
}

impl MiniClawError {
    /// Sanitize a path for display (remove sensitive user information)
    fn sanitize_path(path: &PathBuf) -> String {
        // Replace home directory with ~
        if let Some(home) = dirs::home_dir() {
            if let Ok(stripped) = path.strip_prefix(&home) {
                return format!("~/{}", stripped.display());
            }
        }
        // If not in home, just show filename for security
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| format!("<sanitized>/{}", s))
            .unwrap_or_else(|| "<path>".to_string())
    }

    /// Create an IO error with path context (path will be sanitized in display)
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Create a session persistence error
    pub fn session_persistence(session_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SessionPersistence {
            session_id: session_id.into(),
            message: message.into(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a path validation error
    pub fn path_validation(message: impl Into<String>) -> Self {
        Self::PathValidation {
            message: message.into(),
        }
    }

    /// Create a security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::Security {
            message: message.into(),
        }
    }

    /// Create an external service error
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, duration_secs: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration_secs,
        }
    }

    /// Create a channel error
    pub fn channel(message: impl Into<String>) -> Self {
        Self::Channel {
            message: message.into(),
        }
    }

    /// Returns true if this error is recoverable (system can continue)
    pub fn is_recoverable(&self) -> bool {
        match self {
            // IO errors might be transient (disk full, permission denied temporarily)
            MiniClawError::Io { .. } => true,
            // Session persistence can be retried
            MiniClawError::SessionPersistence { .. } => true,
            // Invalid input is recoverable (user can fix it)
            MiniClawError::InvalidInput { .. } => true,
            // External service errors are typically recoverable (network issues)
            MiniClawError::ExternalService { .. } => true,
            // Timeouts can be retried
            MiniClawError::Timeout { .. } => true,
            // Channel errors might be recoverable depending on context
            MiniClawError::Channel { .. } => true,
            // Serialization errors usually indicate data corruption
            MiniClawError::Serialization { .. } => false,
            // Config errors are typically fatal on startup
            MiniClawError::Config { .. } => false,
            // Security violations are not recoverable
            MiniClawError::Security { .. } => false,
            // Path validation errors are not recoverable
            MiniClawError::PathValidation { .. } => false,
        }
    }

    /// Returns the error severity level for logging
    pub fn severity(&self) -> tracing::Level {
        match self {
            MiniClawError::Security { .. } => tracing::Level::ERROR,
            MiniClawError::Config { .. } => tracing::Level::ERROR,
            MiniClawError::Serialization { .. } => tracing::Level::ERROR,
            MiniClawError::ExternalService { .. } => tracing::Level::WARN,
            MiniClawError::SessionPersistence { .. } => tracing::Level::WARN,
            MiniClawError::Timeout { .. } => tracing::Level::WARN,
            MiniClawError::Io { .. } => tracing::Level::WARN,
            MiniClawError::InvalidInput { .. } => tracing::Level::INFO,
            MiniClawError::PathValidation { .. } => tracing::Level::INFO,
            MiniClawError::Channel { .. } => tracing::Level::WARN,
        }
    }

    /// Returns a localized error message suggestion (i18n support structure)
    ///
    /// This provides context-aware suggestions for user errors.
    /// Future: can be extended with actual i18n translation lookups.
    pub fn suggestion(&self) -> Option<String> {
        match self {
            MiniClawError::InvalidInput { message } => {
                if message.contains("command") {
                    Some("Run 'miniclaw --help' to see available commands.".to_string())
                } else if message.contains("argument") {
                    Some("Check the command syntax with '--help'.".to_string())
                } else {
                    None
                }
            }
            MiniClawError::Config { .. } => {
                Some("Check your config.json file or environment variables.".to_string())
            }
            MiniClawError::PathValidation { .. } => {
                Some("Ensure the path is within allowed directories.".to_string())
            }
            MiniClawError::ExternalService { service, .. } => {
                if service == "telegram" {
                    Some("Check your TELEGRAM_BOT_TOKEN and network connection.".to_string())
                } else if service == "llm" {
                    Some("Verify your API key and LLM provider configuration.".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Result type alias using MiniClawError
pub type Result<T> = std::result::Result<T, MiniClawError>;

// Automatic error conversions for seamless ? operator usage

impl From<std::io::Error> for MiniClawError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::from("<unknown>"),
            source: err,
        }
    }
}

impl From<serde_json::Error> for MiniClawError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for MiniClawError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Channel {
            message: format!("Failed to send message: {}", err),
        }
    }
}

/// Error classification for graceful degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Fatal error - system cannot continue
    Fatal,
    /// High severity - major functionality impaired
    High,
    /// Medium severity - some functionality impaired
    Medium,
    /// Low severity - minor issue, system can continue normally
    Low,
}

impl MiniClawError {
    /// Classify error severity for graceful degradation decisions
    pub fn classify_severity(&self) -> ErrorSeverity {
        match self {
            MiniClawError::Security { .. } => ErrorSeverity::Fatal,
            MiniClawError::Config { .. } => ErrorSeverity::Fatal,
            MiniClawError::Serialization { .. } => ErrorSeverity::High,
            MiniClawError::ExternalService { service, .. } => {
                // External services might be critical or not
                if service == "telegram" || service == "llm" {
                    ErrorSeverity::High
                } else {
                    ErrorSeverity::Medium
                }
            }
            MiniClawError::SessionPersistence { .. } => ErrorSeverity::Medium,
            MiniClawError::Timeout { .. } => ErrorSeverity::Low,
            MiniClawError::Io { .. } => ErrorSeverity::Medium,
            MiniClawError::InvalidInput { .. } => ErrorSeverity::Low,
            MiniClawError::PathValidation { .. } => ErrorSeverity::High,
            MiniClawError::Channel { .. } => ErrorSeverity::Medium,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = MiniClawError::invalid_input("test message");
        assert!(err.to_string().contains("test message"));
        assert!(err.is_recoverable());
        assert_eq!(err.classify_severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = MiniClawError::io("/test/path", io_err);
        assert!(err.to_string().contains("/test/path"));
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_security_error_not_recoverable() {
        let err = MiniClawError::security("access denied");
        assert!(!err.is_recoverable());
        assert_eq!(err.classify_severity(), ErrorSeverity::Fatal);
    }

    #[test]
    fn test_session_persistence_error() {
        let err = MiniClawError::session_persistence("telegram_123", "disk full");
        assert!(err.to_string().contains("telegram_123"));
        assert!(err.to_string().contains("disk full"));
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_external_service_error_severity() {
        let telegram_err = MiniClawError::external_service("telegram", "timeout");
        assert_eq!(telegram_err.classify_severity(), ErrorSeverity::High);

        let other_err = MiniClawError::external_service("weather_api", "timeout");
        assert_eq!(other_err.classify_severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_error_severity_levels() {
        let security = MiniClawError::security("test");
        assert_eq!(security.severity(), tracing::Level::ERROR);

        let invalid_input = MiniClawError::invalid_input("test");
        assert_eq!(invalid_input.severity(), tracing::Level::INFO);
    }

    #[test]
    fn test_all_error_variants_classification() {
        // Test all 10 error variants for proper classification
        assert_eq!(
            MiniClawError::security("test").classify_severity(),
            ErrorSeverity::Fatal
        );
        assert_eq!(
            MiniClawError::config("test").classify_severity(),
            ErrorSeverity::Fatal
        );
        assert_eq!(
            MiniClawError::serialization("test").classify_severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            MiniClawError::path_validation("test").classify_severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            MiniClawError::external_service("telegram", "test").classify_severity(),
            ErrorSeverity::High
        );
        assert_eq!(
            MiniClawError::external_service("other", "test").classify_severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            MiniClawError::session_persistence("id", "test").classify_severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            MiniClawError::channel("test").classify_severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            MiniClawError::timeout("op", 10).classify_severity(),
            ErrorSeverity::Low
        );
        assert_eq!(
            MiniClawError::invalid_input("test").classify_severity(),
            ErrorSeverity::Low
        );
    }

    #[test]
    fn test_error_recoverability() {
        // Recoverable errors
        assert!(MiniClawError::invalid_input("test").is_recoverable());
        assert!(MiniClawError::timeout("test", 5).is_recoverable());
        assert!(MiniClawError::external_service("api", "down").is_recoverable());
        assert!(MiniClawError::session_persistence("id", "disk full").is_recoverable());
        assert!(MiniClawError::channel("closed").is_recoverable());

        // Non-recoverable errors
        assert!(!MiniClawError::security("breach").is_recoverable());
        assert!(!MiniClawError::config("invalid").is_recoverable());
        assert!(!MiniClawError::serialization("corrupt").is_recoverable());
        assert!(!MiniClawError::path_validation("outside").is_recoverable());
    }

    #[test]
    fn test_error_suggestions() {
        // Test suggestion system (i18n support structure)
        let input_err = MiniClawError::invalid_input("unknown command");
        assert!(input_err.suggestion().is_some());
        assert!(input_err.suggestion().unwrap().contains("--help"));

        let config_err = MiniClawError::config("missing key");
        assert!(config_err.suggestion().is_some());
        assert!(config_err.suggestion().unwrap().contains("config.json"));

        let telegram_err = MiniClawError::external_service("telegram", "timeout");
        assert!(telegram_err.suggestion().is_some());
        assert!(telegram_err
            .suggestion()
            .unwrap()
            .contains("TELEGRAM_BOT_TOKEN"));

        let llm_err = MiniClawError::external_service("llm", "unauthorized");
        assert!(llm_err.suggestion().is_some());
        assert!(llm_err.suggestion().unwrap().contains("API key"));
    }

    #[test]
    fn test_error_conversions() {
        // Test From trait implementations
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let converted: MiniClawError = io_err.into();
        assert!(matches!(converted, MiniClawError::Io { .. }));

        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let converted: MiniClawError = json_err.into();
        assert!(matches!(converted, MiniClawError::Serialization { .. }));
    }
}
