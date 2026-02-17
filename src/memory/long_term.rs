//! Long-term memory management
//!
//! This module provides functions and structures for managing long-term memory stored in MEMORY.md

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use tokio::fs;
use tokio::sync::Mutex;

use crate::memory::types::MemoryError;

/// Maximum file size for MEMORY.md (1MB)
const MEMORY_FILE_SIZE_LIMIT: u64 = 1024 * 1024;

/// Timestamp format suffix in memory entries: "(added at YYYY-MM-DDTHH:MM:SSZ)"
const TIMESTAMP_SUFFIX: &str = "(added at ";

/// Checks file size and logs warning if exceeds limit
///
/// # Arguments
/// * `file_path` - Path to the file to check
///
/// # Returns
/// * `Ok(())` - Always returns Ok, logs warning if file is too large
/// * `Err(MemoryError)` - Only for metadata read errors
async fn check_file_size(file_path: &Path) -> Result<(), MemoryError> {
    if file_path.exists() {
        let metadata = fs::metadata(file_path)
            .await
            .map_err(|e| MemoryError::StorageFailed {
                operation: format!("get file metadata for {}", file_path.display()),
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

/// A long-term memory entry with content and timestamp
#[derive(Debug, Clone)]
pub struct LongTermMemoryEntry {
    /// The content of the memory entry
    pub content: String,
    /// When the entry was created (ISO 8601 UTC)
    pub timestamp: DateTime<Utc>,
}

impl LongTermMemoryEntry {
    /// Creates a new long-term memory entry with the current timestamp
    ///
    /// # Arguments
    /// * `content` - The content to store
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
        }
    }

    /// Creates a long-term memory entry with a specific timestamp
    ///
    /// # Arguments
    /// * `content` - The content to store
    /// * `timestamp` - The timestamp to use
    pub fn with_timestamp(content: String, timestamp: DateTime<Utc>) -> Self {
        Self { content, timestamp }
    }

    /// Parses a memory entry from a bullet point line
    ///
    /// Extracts content and timestamp from format: "- Content (added at 2026-02-16T10:30:00Z)"
    ///
    /// # Arguments
    /// * `line` - The bullet point line to parse
    ///
    /// # Returns
    /// * `Some(LongTermMemoryEntry)` - If parsing succeeds
    /// * `None` - If format is invalid
    fn parse_from_line(line: &str) -> Self {
        let content_str = line.trim();
        
        // Try to extract timestamp from "(added at ...)" suffix
        if let Some(timestamp_start) = content_str.rfind(TIMESTAMP_SUFFIX) {
            let content = content_str[..timestamp_start].trim().to_string();
            let timestamp_str = &content_str[timestamp_start + TIMESTAMP_SUFFIX.len()..];
            
            // Extract timestamp until closing ')'
            if let Some(end_paren) = timestamp_str.find(')') {
                let timestamp_value = &timestamp_str[..end_paren];
                
                // Try to parse ISO 8601 timestamp
                if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_value) {
                    return Self {
                        content,
                        timestamp: timestamp.with_timezone(&Utc),
                    };
                }
            }
        }
        
        // Fallback: use entire line as content with current timestamp
        Self::new(content_str.to_string())
    }
}

/// A dated section in the memory file
#[derive(Debug, Clone)]
pub struct MemorySection {
    /// The date of this section (YYYY-MM-DD)
    pub date: NaiveDate,
    /// The entries in this section
    pub entries: Vec<LongTermMemoryEntry>,
}

/// Long-term memory storage using MEMORY.md file
///
/// Thread-safe via Arc<Mutex<>> for write serialization and caching.
/// Stores memory in Markdown format with dated sections (## YYYY-MM-DD) and bullet points.
#[derive(Debug, Clone)]
pub struct LongTermMemory {
    /// Path to the MEMORY.md file
    file_path: PathBuf,
    /// Cache of parsed sections (optional optimization)
    cache: Arc<Mutex<Option<Vec<MemorySection>>>>,
    /// Write lock to prevent concurrent write races
    write_lock: Arc<Mutex<()>>,
}

impl LongTermMemory {
    /// Creates a new LongTermMemory for the given workspace path
    ///
    /// # Arguments
    /// * `workspace_path` - The workspace directory (e.g., ~/.miniclaw/workspace)
    pub fn new(workspace_path: &Path) -> Self {
        let file_path = workspace_path.join("memory").join("MEMORY.md");
        Self {
            file_path,
            cache: Arc::new(Mutex::new(None)),
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Ensures the MEMORY.md file exists with a default template
    ///
    /// Creates parent directories and sets file permissions to 0600.
    ///
    /// # Returns
    /// * `Ok(())` - File exists or was created successfully
    /// * `Err(MemoryError)` - If creation fails
    pub async fn ensure_exists(&self) -> Result<(), MemoryError> {
        if !self.file_path.exists() {
            // Create parent directories
            if let Some(parent) = self.file_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| MemoryError::StorageFailed {
                        operation: format!("create memory directory {}", parent.display()),
                        source: e,
                    })?;
            }

            // Create default template
            let template = "# Memory\n\n";
            fs::write(&self.file_path, template)
                .await
                .map_err(|e| MemoryError::StorageFailed {
                    operation: format!("create memory file {}", self.file_path.display()),
                    source: e,
                })?;

            // Set permissions to 0600 (owner read/write only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(&self.file_path, perms)
                    .map_err(|e| MemoryError::StorageFailed {
                        operation: format!("set file permissions for {}", self.file_path.display()),
                        source: e,
                    })?;
            }

            tracing::info!("Created long-term memory file: {}", self.file_path.display());
        }

        Ok(())
    }

    /// Appends content to long-term memory
    ///
    /// Groups entries by date (## YYYY-MM-DD) and adds bullet points.
    /// Creates the file if it doesn't exist.
    /// Thread-safe: uses write lock to prevent concurrent modifications.
    ///
    /// # Arguments
    /// * `content` - The content to append
    ///
    /// # Returns
    /// * `Ok(())` - Content appended successfully
    /// * `Err(MemoryError)` - If append fails
    pub async fn append_entry(&self, content: &str) -> Result<(), MemoryError> {
        if content.trim().is_empty() {
            return Err(MemoryError::InvalidContent("Content cannot be empty".to_string()));
        }

        // Acquire write lock to prevent concurrent modifications
        let _lock = self.write_lock.lock().await;

        self.ensure_exists().await?;

        // Check file size
        if let Err(e) = check_file_size(&self.file_path).await {
            tracing::warn!("Memory file size check: {}", e);
        }

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let timestamp = Utc::now().to_rfc3339();
        let entry_line = format!("- {} {} {})\n", content.trim(), TIMESTAMP_SUFFIX, timestamp);

        // Read existing content
        let existing = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| MemoryError::StorageFailed {
                operation: format!("read memory file {}", self.file_path.display()),
                source: e,
            })?;

        // Check if today's section exists
        let today_header = format!("## {}", today);
        let new_content = if existing.contains(&today_header) {
            // Append to existing today section - use replacen to replace only first occurrence
            existing.replacen(
                &format!("{}\n", today_header),
                &format!("{}\n{}", today_header, entry_line),
                1,
            )
        } else {
            // Create new today section at end
            format!("{}\n{}\n{}", existing.trim(), today_header, entry_line)
        };

        // Write updated content
        fs::write(&self.file_path, new_content)
            .await
            .map_err(|e| MemoryError::StorageFailed {
                operation: format!("write memory file {}", self.file_path.display()),
                source: e,
            })?;

        // Set permissions again in case file was recreated
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&self.file_path, perms);
        }

        // Invalidate cache
        let mut cache = self.cache.lock().await;
        *cache = None;

        tracing::debug!(
            file = %self.file_path.display(),
            date = %today,
            "Added entry to long-term memory"
        );

        Ok(())
    }

    /// Reads all memory sections from the file
    ///
    /// Parses the Markdown structure and extracts dated sections.
    /// Returns empty Vec if file doesn't exist.
    ///
    /// # Returns
    /// * `Ok(Vec<MemorySection>)` - All memory sections
    /// * `Err(MemoryError)` - If reading fails
    pub async fn read_all(&self) -> Result<Vec<MemorySection>, MemoryError> {
        // Check cache first
        let cache = self.cache.lock().await;
        if let Some(ref sections) = *cache {
            return Ok(sections.clone());
        }
        drop(cache);

        if !self.file_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| MemoryError::StorageFailed {
                operation: format!("read memory file {}", self.file_path.display()),
                source: e,
            })?;

        let sections = Self::parse_memory_content(&content)?;

        // Update cache
        let mut cache = self.cache.lock().await;
        *cache = Some(sections.clone());

        Ok(sections)
    }

    /// Reads today's entries only
    ///
    /// Filters entries by current date (YYYY-MM-DD).
    /// Returns empty Vec if no entries for today.
    ///
    /// # Returns
    /// * `Ok(Vec<LongTermMemoryEntry>)` - Today's entries
    /// * `Err(MemoryError)` - If reading fails
    pub async fn read_today(&self) -> Result<Vec<LongTermMemoryEntry>, MemoryError> {
        let all = self.read_all().await?;
        let today = Utc::now().date_naive();

        Ok(all
            .into_iter()
            .find(|section| section.date == today)
            .map(|section| section.entries)
            .unwrap_or_default())
    }

    /// Parses memory content from Markdown format
    ///
    /// Extracts ## YYYY-MM-DD sections and bullet points with timestamps.
    fn parse_memory_content(content: &str) -> Result<Vec<MemorySection>, MemoryError> {
        let mut sections = Vec::new();
        let mut current_section: Option<MemorySection> = None;

        for line in content.lines() {
            if let Some(date_str) = line.strip_prefix("## ") {
                // Save previous section if exists
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                // Parse date from header
                if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    current_section = Some(MemorySection {
                        date,
                        entries: Vec::new(),
                    });
                }
            } else if let Some(entry_content) = line.strip_prefix("- ") {
                if let Some(ref mut section) = current_section {
                    // Parse bullet point entry with timestamp extraction
                    let entry = LongTermMemoryEntry::parse_from_line(entry_content);
                    section.entries.push(entry);
                }
            }
        }

        // Don't forget last section
        if let Some(section) = current_section {
            sections.push(section);
        }

        Ok(sections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_long_term_memory_creation() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        assert!(!memory.file_path.exists());
    }

    #[tokio::test]
    async fn test_ensure_exists_creates_file() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.ensure_exists().await.unwrap();
        
        assert!(memory.file_path.exists());
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        assert!(content.contains("# Memory"));
    }

    #[tokio::test]
    async fn test_ensure_exists_creates_directories() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().join("nested").join("workspace");
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.ensure_exists().await.unwrap();
        
        assert!(memory.file_path.parent().unwrap().exists());
        assert!(memory.file_path.exists());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_file_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;
        
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // Create file via append_entry
        memory.append_entry("Test entry").await.unwrap();
        
        // Check permissions are 0600
        let metadata = std::fs::metadata(&memory.file_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        // Check that permissions are 0600 (owner read/write only)
        // Mode includes file type bits, so we mask with 0o777
        assert_eq!(mode & 0o777, 0o600, "File permissions should be 0600");
    }

    #[tokio::test]
    async fn test_append_entry_creates_file() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("Test content").await.unwrap();
        
        assert!(memory.file_path.exists());
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        assert!(content.contains("Test content"));
        assert!(content.contains("## "));
    }

    #[tokio::test]
    async fn test_append_entry_empty_content_fails() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        let result = memory.append_entry("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append_entry_whitespace_only_fails() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        let result = memory.append_entry("   ").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_append_entry_adds_date_header() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("First entry").await.unwrap();
        
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("## {}", today)));
    }

    #[tokio::test]
    async fn test_append_entry_multiple_same_day() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("First entry").await.unwrap();
        memory.append_entry("Second entry").await.unwrap();
        
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        let today_header_count = content.matches("## ").count();
        assert_eq!(today_header_count, 1); // Only one date header
        assert!(content.contains("First entry"));
        assert!(content.contains("Second entry"));
    }

    #[tokio::test]
    async fn test_read_all_empty_file() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.ensure_exists().await.unwrap();
        let sections = memory.read_all().await.unwrap();
        
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_read_all_missing_file() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        let sections = memory.read_all().await.unwrap();
        
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_read_all_parses_sections() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // Ensure directory exists first
        memory.ensure_exists().await.unwrap();
        
        // Create file with multiple sections manually
        let content = r#"# Memory

## 2026-02-14
- Entry 1 (added at 2026-02-14T10:00:00Z)
- Entry 2 (added at 2026-02-14T11:00:00Z)

## 2026-02-15
- Entry 3 (added at 2026-02-15T10:00:00Z)
- Entry 4 (added at 2026-02-15T11:00:00Z)
- Entry 5 (added at 2026-02-15T12:00:00Z)
"#;
        fs::write(&memory.file_path, content).await.unwrap();
        
        let sections = memory.read_all().await.unwrap();
        
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].date.to_string(), "2026-02-14");
        assert_eq!(sections[0].entries.len(), 2);
        assert_eq!(sections[1].date.to_string(), "2026-02-15");
        assert_eq!(sections[1].entries.len(), 3);
        
        // Verify timestamps were parsed correctly
        assert_eq!(sections[0].entries[0].content, "Entry 1");
        assert_eq!(
            sections[0].entries[0].timestamp.to_rfc3339(),
            "2026-02-14T10:00:00+00:00"
        );
    }

    #[tokio::test]
    async fn test_read_today_no_entries() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // Ensure directory exists first
        memory.ensure_exists().await.unwrap();
        
        // Create file with only old entries
        let content = r#"# Memory

## 2026-02-14
- Old entry (added at 2026-02-14T10:00:00Z)
"#;
        fs::write(&memory.file_path, content).await.unwrap();
        
        let entries = memory.read_today().await.unwrap();
        
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_read_today_with_entries() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("Today's entry").await.unwrap();
        
        let entries = memory.read_today().await.unwrap();
        
        assert_eq!(entries.len(), 1);
        assert!(entries[0].content.contains("Today's entry"));
    }

    #[tokio::test]
    async fn test_parse_memory_content_empty() {
        let sections = LongTermMemory::parse_memory_content("").unwrap();
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_parse_memory_content_no_dates() {
        let content = "# Memory\n\nJust some text\n";
        let sections = LongTermMemory::parse_memory_content(content).unwrap();
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_parse_memory_content_single_section() {
        let content = r#"# Memory

## 2026-02-16
- Entry 1 (added at 2026-02-16T10:00:00Z)
- Entry 2 (added at 2026-02-16T11:00:00Z)
"#;
        let sections = LongTermMemory::parse_memory_content(content).unwrap();
        
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].entries.len(), 2);
    }

    #[tokio::test]
    async fn test_parse_memory_content_invalid_date() {
        let content = r#"# Memory

## Not-A-Date
- Entry 1

## 2026-02-16
- Entry 2 (added at 2026-02-16T10:00:00Z)
"#;
        let sections = LongTermMemory::parse_memory_content(content).unwrap();
        
        // Should skip invalid date and parse valid one
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].date.to_string(), "2026-02-16");
    }

    #[tokio::test]
    async fn test_entry_has_timestamp() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("Test with timestamp").await.unwrap();
        
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        assert!(content.contains("(added at "));
        assert!(content.contains("T")); // ISO 8601 format contains 'T'
    }

    #[tokio::test]
    async fn test_concurrent_read() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // Add some entries first
        for i in 0..5 {
            memory.append_entry(&format!("Entry {}", i)).await.unwrap();
        }
        
        let memory1 = memory.clone();
        let memory2 = memory.clone();
        let memory3 = memory.clone();
        
        // Concurrent reads should work fine
        let handle1 = tokio::spawn(async move {
            memory1.read_all().await.unwrap()
        });
        
        let handle2 = tokio::spawn(async move {
            memory2.read_today().await.unwrap()
        });
        
        let handle3 = tokio::spawn(async move {
            memory3.read_all().await.unwrap()
        });
        
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();
        let result3 = handle3.await.unwrap();
        
        // All reads should return consistent results
        assert!(!result1.is_empty());
        assert!(!result2.is_empty());
        assert!(!result3.is_empty());
        
        let total1: usize = result1.iter().map(|s| s.entries.len()).sum();
        let total3: usize = result3.iter().map(|s| s.entries.len()).sum();
        assert_eq!(total1, total3);
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        let memory1 = memory.clone();
        let memory2 = memory.clone();
        let memory3 = memory.clone();
        
        // Concurrent writes should be safe due to write_lock
        let handle1 = tokio::spawn(async move {
            for i in 0..5 {
                memory1.append_entry(&format!("Task 1 Entry {}", i)).await.unwrap();
            }
        });
        
        let handle2 = tokio::spawn(async move {
            for i in 0..5 {
                memory2.append_entry(&format!("Task 2 Entry {}", i)).await.unwrap();
            }
        });
        
        let handle3 = tokio::spawn(async move {
            for i in 0..5 {
                memory3.append_entry(&format!("Task 3 Entry {}", i)).await.unwrap();
            }
        });
        
        handle1.await.unwrap();
        handle2.await.unwrap();
        handle3.await.unwrap();
        
        // All 15 entries should be present
        let entries = memory.read_today().await.unwrap();
        assert_eq!(entries.len(), 15);
    }

    #[tokio::test]
    async fn test_unicode_content() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("日本語テスト").await.unwrap();
        memory.append_entry("🎉 Emoji test 🚀").await.unwrap();
        
        let entries = memory.read_today().await.unwrap();
        
        assert_eq!(entries.len(), 2);
        // Check that both entries exist (order may vary)
        let contents: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
        assert!(contents.iter().any(|c| c.contains("日本語")), "Missing Japanese content");
        assert!(contents.iter().any(|c| c.contains("🎉")), "Missing emoji content");
    }

    #[tokio::test]
    async fn test_memory_section_struct() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let entry = LongTermMemoryEntry::new("Test".to_string());
        
        let section = MemorySection {
            date,
            entries: vec![entry],
        };
        
        assert_eq!(section.date.to_string(), "2026-02-16");
        assert_eq!(section.entries.len(), 1);
    }

    #[tokio::test]
    async fn test_long_term_memory_entry_new() {
        let entry = LongTermMemoryEntry::new("Test content".to_string());
        
        assert_eq!(entry.content, "Test content");
        assert!(entry.timestamp.timestamp() > 0);
    }

    #[tokio::test]
    async fn test_long_term_memory_entry_with_timestamp() {
        let timestamp = DateTime::parse_from_rfc3339("2026-02-14T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let entry = LongTermMemoryEntry::with_timestamp("Test".to_string(), timestamp);
        
        assert_eq!(entry.content, "Test");
        assert_eq!(entry.timestamp.to_rfc3339(), "2026-02-14T10:30:00+00:00");
    }

    #[tokio::test]
    async fn test_parse_from_line_with_timestamp() {
        let line = "Entry content (added at 2026-02-14T10:30:00Z)";
        let entry = LongTermMemoryEntry::parse_from_line(line);
        
        assert_eq!(entry.content, "Entry content");
        assert_eq!(entry.timestamp.to_rfc3339(), "2026-02-14T10:30:00+00:00");
    }

    #[tokio::test]
    async fn test_parse_from_line_without_timestamp() {
        let line = "Entry without timestamp";
        let entry = LongTermMemoryEntry::parse_from_line(line);
        
        assert_eq!(entry.content, "Entry without timestamp");
        // Timestamp should be current time (we can't test exact value)
        assert!(entry.timestamp.timestamp() > 0);
    }

    #[tokio::test]
    async fn test_append_trims_content() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("  Trimmed content  ").await.unwrap();
        
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        assert!(content.contains("Trimmed content"));
        // Should not contain the extra spaces around
        assert!(!content.contains("  Trimmed content  "));
    }

    #[tokio::test]
    async fn test_clone_behavior() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        memory.append_entry("Original").await.unwrap();
        
        let cloned = memory.clone();
        cloned.append_entry("Via clone").await.unwrap();
        
        let entries = memory.read_today().await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // First read to populate cache
        memory.append_entry("First").await.unwrap();
        let _ = memory.read_all().await.unwrap();
        
        // Append should invalidate cache
        memory.append_entry("Second").await.unwrap();
        
        // Read again - should get fresh data
        let sections = memory.read_all().await.unwrap();
        let total_entries: usize = sections.iter().map(|s| s.entries.len()).sum();
        assert_eq!(total_entries, 2);
    }

    #[tokio::test]
    async fn test_replacen_prevents_duplicate_headers() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path().to_path_buf();
        let memory = LongTermMemory::new(&workspace_path);
        
        // Add entries
        memory.append_entry("Entry 1").await.unwrap();
        memory.append_entry("Entry 2").await.unwrap();
        memory.append_entry("Entry 3").await.unwrap();
        
        let content = fs::read_to_string(&memory.file_path).await.unwrap();
        
        // Should only have ONE date header for today
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let header_count = content.matches(&format!("## {}", today)).count();
        assert_eq!(header_count, 1, "Should have exactly one date header");
        
        // All three entries should be present
        assert!(content.contains("Entry 1"));
        assert!(content.contains("Entry 2"));
        assert!(content.contains("Entry 3"));
    }
}