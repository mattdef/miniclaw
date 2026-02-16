//! Long-term memory management
//!
//! This module provides functions for managing long-term memory stored in MEMORY.md

use std::path::{Path, PathBuf};

use chrono::Utc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::memory::types::{MemoryEntry, MemoryError, MemoryType};

/// Maximum file size for MEMORY.md (1MB)
const MEMORY_FILE_SIZE_LIMIT: u64 = 1024 * 1024;

/// Appends content to long-term memory (MEMORY.md)
///
/// # Arguments
/// * `workspace_path` - The workspace directory
/// * `content` - The content to append
/// * `store_entry` - Optional callback to store entry in short-term memory
///
/// # Returns
/// * `Ok((String, MemoryEntry))` - Path to the memory file and the created entry
/// * `Err(MemoryError)` - If operation fails
pub async fn append_to_memory<F>(
    workspace_path: &Path,
    content: String,
    store_entry: Option<F>,
) -> Result<(String, MemoryEntry), MemoryError>
where
    F: FnOnce(MemoryEntry) -> (),
{
    // Validate content
    if content.trim().is_empty() {
        return Err(MemoryError::InvalidContent("Content cannot be empty".to_string()));
    }
    
    tracing::info!("Appending content to long-term memory (MEMORY.md)");
    
    // Create memory directory if it doesn't exist
    let memory_dir = workspace_path.join("memory");
    fs::create_dir_all(&memory_dir)
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "create memory directory".to_string(),
            source: e,
        })?;
    
    // Create MEMORY.md path
    let memory_file = memory_dir.join("MEMORY.md");
    
    // Check file size before writing (log warning but continue)
    if let Err(e) = check_file_size(&memory_file).await {
        tracing::warn!("Memory file size check: {}", e);
    }
    
    // Format content with timestamp
    let timestamp = Utc::now();
    let formatted_content = format!(
        "## {}\n\n{}\n\n---\n\n",
        timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
        content
    );
    
    // Append to file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_file)
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "open memory file for appending".to_string(),
            source: e,
        })?;
    
    file.write_all(formatted_content.as_bytes())
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "write to memory file".to_string(),
            source: e,
        })?;
    
    tracing::info!("Successfully appended to long-term memory: {}", memory_file.display());
    
    // Create memory entry for short-term storage
    let entry = MemoryEntry {
        content: content.clone(),
        timestamp,
        memory_type: MemoryType::LongTerm,
        file_path: Some(memory_file.to_string_lossy().to_string()),
    };
    
    // Store in short-term memory if callback provided
    if let Some(store_fn) = store_entry {
        store_fn(entry.clone());
    }
    
    Ok((memory_file.to_string_lossy().to_string(), entry))
}

/// Checks file size and logs warning if exceeds limit
///
/// # Arguments
/// * `file_path` - Path to the file to check
///
/// # Returns
/// * `Ok(())` - Always returns Ok, logs warning if file is too large
/// * `Err(MemoryError)` - Only for metadata read errors
pub async fn check_file_size(file_path: &PathBuf) -> Result<(), MemoryError> {
    if file_path.exists() {
        let metadata = fs::metadata(file_path)
            .await
            .map_err(|e| MemoryError::StorageFailed {
                operation: "get file metadata".to_string(),
                source: e,
            })?;
        
        let size = metadata.len();
        if size > MEMORY_FILE_SIZE_LIMIT {
            tracing::warn!(
                "Memory file '{}' exceeds size limit: {} bytes (limit: {} bytes). Consider memory maintenance.",
                file_path.display(),
                size,
                MEMORY_FILE_SIZE_LIMIT
            );
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_append_to_memory() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        
        let result = append_to_memory(&workspace_path, "Test content".to_string(), None::<fn(MemoryEntry)>).await;
        assert!(result.is_ok());
        
        let (file_path, _entry) = result.unwrap();
        assert!(file_path.contains("MEMORY.md"));
        
        // Check that file was created
        let memory_file = workspace_path.join("memory").join("MEMORY.md");
        assert!(memory_file.exists());
    }

    #[tokio::test]
    async fn test_empty_content() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        
        let result = append_to_memory(&workspace_path, "".to_string(), None::<fn(MemoryEntry)>).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_file_size_check() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory_file = workspace_path.join("MEMORY.md");
        
        // Create a file larger than 1MB
        let large_content = "x".repeat(1024 * 1024 + 1000);
        fs::write(&memory_file, large_content).await.unwrap();
        
        // Should now return Ok but log warning
        let result = check_file_size(&memory_file).await;
        assert!(result.is_ok());
    }
}