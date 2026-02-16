//! Types for the memory system
//!
//! This module defines the core types for memory management including:
//! - MemoryEntry struct for storing memory entries
//! - MemoryType enum for categorizing memory
//! - MemoryError enum for error handling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of memory entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Long-term memory stored in MEMORY.md
    LongTerm,
    /// Daily note stored in YYYY-MM-DD.md
    Daily,
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryType::LongTerm => write!(f, "long_term"),
            MemoryType::Daily => write!(f, "daily"),
        }
    }
}

/// A memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// The content of the memory entry
    pub content: String,
    /// When the entry was created
    pub timestamp: DateTime<Utc>,
    /// Type of memory entry
    pub memory_type: MemoryType,
    /// Path to the file where this entry is stored (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Error types for memory operations
#[derive(Debug)]
pub enum MemoryError {
    /// File not found
    FileNotFound(String),
    /// File too large
    FileTooLarge { path: String, size: u64, limit: u64 },
    /// Invalid content
    InvalidContent(String),
    /// Storage operation failed
    StorageFailed {
        operation: String,
        source: std::io::Error,
    },
    /// Path validation failed
    PathValidationFailed(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::FileNotFound(path) => {
                write!(f, "Memory file not found: {}", path)
            }
            MemoryError::FileTooLarge { path, size, limit } => {
                write!(
                    f,
                    "Memory file '{}' exceeds size limit: {} bytes (limit: {} bytes)",
                    path, size, limit
                )
            }
            MemoryError::InvalidContent(msg) => {
                write!(f, "Invalid content: {}", msg)
            }
            MemoryError::StorageFailed { operation, source } => {
                write!(f, "Storage operation '{}' failed: {}", operation, source)
            }
            MemoryError::PathValidationFailed(msg) => {
                write!(f, "Path validation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for MemoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_display() {
        assert_eq!(MemoryType::LongTerm.to_string(), "long_term");
        assert_eq!(MemoryType::Daily.to_string(), "daily");
    }

    #[test]
    fn test_memory_error_display() {
        let error = MemoryError::FileNotFound("test.md".to_string());
        assert!(error.to_string().contains("test.md"));

        let error = MemoryError::FileTooLarge {
            path: "test.md".to_string(),
            size: 2000,
            limit: 1000,
        };
        assert!(error.to_string().contains("2000"));
        assert!(error.to_string().contains("1000"));
    }
}
