//! Utility modules for cross-cutting concerns

pub mod paths;

// Re-export commonly used items
pub use paths::{canonicalize_base_dir, is_system_path, validate_path, PathValidationError};
