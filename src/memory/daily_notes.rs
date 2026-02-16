//! Daily notes management
//!
//! This module provides functions for managing daily notes stored in YYYY-MM-DD.md files

use std::path::Path;

use chrono::Utc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::memory::types::{MemoryEntry, MemoryError, MemoryType};

/// Creates a daily note
///
/// # Arguments
/// * `workspace_path` - The workspace directory
/// * `content` - The content to store
/// * `store_entry` - Optional callback to store entry in short-term memory
///
/// # Returns
/// * `Ok((String, MemoryEntry))` - Path to the daily note file and the created entry
/// * `Err(MemoryError)` - If operation fails
pub async fn create_daily_note<F>(
    workspace_path: &Path,
    content: String,
    store_entry: Option<F>,
) -> Result<(String, MemoryEntry), MemoryError>
where
    F: FnOnce(MemoryEntry),
{
    // Validate content
    if content.trim().is_empty() {
        return Err(MemoryError::InvalidContent("Content cannot be empty".to_string()));
    }
    
    tracing::info!("Creating daily note");
    
    // Create memory directory if it doesn't exist
    let memory_dir = workspace_path.join("memory");
    fs::create_dir_all(&memory_dir)
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "create memory directory".to_string(),
            source: e,
        })?;
    
    // Create daily note filename
    let today = Utc::now().format("%Y-%m-%d");
    let daily_file = memory_dir.join(format!("{}.md", today));
    
    // Format content with timestamp
    let timestamp = Utc::now();
    let formatted_content = format!(
        "# Daily Note - {}\n\n## {}\n\n{}\n\n---\n\n",
        today,
        timestamp.format("%H:%M:%S UTC"),
        content
    );
    
    // Append to file (don't overwrite existing entries)
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&daily_file)
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "open daily note for appending".to_string(),
            source: e,
        })?;
    
    file.write_all(formatted_content.as_bytes())
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: "write daily note".to_string(),
            source: e,
        })?;
    
    tracing::info!("Successfully created daily note: {}", daily_file.display());
    
    // Create memory entry for short-term storage
    let entry = MemoryEntry {
        content: content.clone(),
        timestamp,
        memory_type: MemoryType::Daily,
        file_path: Some(daily_file.to_string_lossy().to_string()),
    };
    
    // Store in short-term memory if callback provided
    if let Some(store_fn) = store_entry {
        store_fn(entry.clone());
    }
    
    Ok((daily_file.to_string_lossy().to_string(), entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_daily_note() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        
        let result = create_daily_note(&workspace_path, "Test daily note".to_string(), None::<fn(MemoryEntry)>).await;
        assert!(result.is_ok());
        
        let (file_path, _entry) = result.unwrap();
        assert!(file_path.contains(".md"));
        
        // Check that file was created
        let today = Utc::now().format("%Y-%m-%d");
        let daily_file = workspace_path.join("memory").join(format!("{}.md", today));
        assert!(daily_file.exists());
    }

    #[tokio::test]
    async fn test_empty_daily_note() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        
        let result = create_daily_note(&workspace_path, "".to_string(), None::<fn(MemoryEntry)>).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }
}