//! Utility modules for cross-cutting concerns

pub mod circuit_breaker;
pub mod error;
pub mod fs;
pub mod paths;
pub mod security;

// Re-export commonly used items
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use error::{ErrorSeverity, MiniClawError, Result};
pub use paths::{PathValidationError, canonicalize_base_dir, is_system_path, validate_path};
pub use security::{WhitelistChecker, is_command_allowed};
