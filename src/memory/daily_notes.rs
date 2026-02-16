//! Daily notes management
//!
//! This module provides functions for managing daily notes stored in YYYY-MM-DD.md files

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::memory::types::{MemoryEntry, MemoryError, MemoryType};

/// Default number of days to show for `memory recent` command
pub const DEFAULT_RECENT_DAYS: usize = 7;

/// Number of days to retain daily notes before cleanup
pub const DAILY_NOTE_RETENTION_DAYS: i64 = 30;

/// A single entry in a daily note file
#[derive(Debug, Clone, PartialEq)]
pub struct DailyNoteEntry {
    /// Timestamp of the entry
    pub timestamp: DateTime<Utc>,
    /// Content of the entry
    pub content: String,
}

/// A section of daily notes for a specific date
#[derive(Debug, Clone, PartialEq)]
pub struct DailyNoteSection {
    /// Date of the section
    pub date: NaiveDate,
    /// Path to the file
    pub file_path: PathBuf,
    /// Entries for this date
    pub entries: Vec<DailyNoteEntry>,
}

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

/// Reads daily notes from the last N days
///
/// # Arguments
/// * `workspace_path` - The workspace directory
/// * `days` - Number of days to read (going backwards from today)
///
/// # Returns
/// * `Ok(Vec<DailyNoteSection>)` - List of daily note sections, sorted chronologically
/// * `Err(MemoryError)` - If operation fails
pub async fn read_recent_days(
    workspace_path: &Path,
    days: usize,
) -> Result<Vec<DailyNoteSection>, MemoryError> {
    // Validate days parameter
    if days == 0 {
        return Ok(Vec::new());
    }

    tracing::debug!(days = days, "Reading recent daily notes");

    let memory_dir = workspace_path.join("memory");
    let end_date = Utc::now().date_naive();

    let mut sections = Vec::new();

    // Iterate through date range (going backwards from today)
    for day_offset in 0..days {
        let date = end_date - Duration::days(day_offset as i64);
        let filename = format!("{}.md", date.format("%Y-%m-%d"));
        let file_path = memory_dir.join(&filename);

        if file_path.exists() {
            match parse_daily_note_file(&file_path).await {
                Ok(entries) => {
                    if !entries.is_empty() {
                        sections.push(DailyNoteSection {
                            date,
                            file_path,
                            entries,
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(file = %file_path.display(), error = %e, "Failed to parse daily note file");
                    // Continue with other files even if one fails
                }
            }
        }
    }

    // Sort chronologically (oldest first)
    sections.sort_by(|a, b| a.date.cmp(&b.date));

    tracing::info!(
        days = days,
        sections_found = sections.len(),
        total_entries = sections.iter().map(|s| s.entries.len()).sum::<usize>(),
        "Completed reading recent daily notes"
    );

    Ok(sections)
}

/// Parses a daily note file and extracts entries
///
/// # Arguments
/// * `file_path` - Path to the daily note file
///
/// # Returns
/// * `Ok(Vec<DailyNoteEntry>)` - List of entries in the file
/// * `Err(MemoryError)` - If parsing fails
async fn parse_daily_note_file(file_path: &Path) -> Result<Vec<DailyNoteEntry>, MemoryError> {
    let content = fs::read_to_string(file_path)
        .await
        .map_err(|e| MemoryError::StorageFailed {
            operation: format!("read daily note file {}", file_path.display()),
            source: e,
        })?;

    // Extract date from filename (YYYY-MM-DD.md)
    let file_date = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|name| NaiveDate::parse_from_str(name, "%Y-%m-%d").ok())
        .ok_or_else(|| MemoryError::StorageFailed {
            operation: format!("parse date from filename {}", file_path.display()),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid filename format"),
        })?;

    let mut entries = Vec::new();
    let mut current_timestamp: Option<DateTime<Utc>> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        // Check for timestamp header (## HH:MM:SS UTC)
        if line.starts_with("## ") && line.contains(":") && line.contains("UTC") {
            // Save previous entry if exists
            if let Some(ts) = current_timestamp {
                if !current_content.trim().is_empty() {
                    entries.push(DailyNoteEntry {
                        timestamp: ts,
                        content: current_content.trim().to_string(),
                    });
                }
            }

            // Parse timestamp from line like "## 14:30:00 UTC"
            let time_str = line.trim_start_matches("## ").trim();
            current_timestamp = parse_time_string(time_str, file_date);
            current_content.clear();
        } else if !line.starts_with("# ") && !line.starts_with("---") && !line.trim().is_empty() {
            // Accumulate content (skip title lines and separators)
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(line);
        }
    }

    // Don't forget the last entry
    if let Some(ts) = current_timestamp {
        if !current_content.trim().is_empty() {
            entries.push(DailyNoteEntry {
                timestamp: ts,
                content: current_content.trim().to_string(),
            });
        }
    }

    Ok(entries)
}

/// Parses a time string in format "HH:MM:SS UTC" with a given date
fn parse_time_string(time_str: &str, date: NaiveDate) -> Option<DateTime<Utc>> {
    // Parse time string like "14:30:00 UTC"
    let parts: Vec<&str> = time_str.split_whitespace().collect();
    if parts.len() != 2 || parts[1] != "UTC" {
        return None;
    }

    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }

    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    let second: u32 = time_parts[2].parse().ok()?;

    // Combine provided date with parsed time
    date
        .and_hms_opt(hour, minute, second)
        .map(|naive| DateTime::from_naive_utc_and_offset(naive, Utc))
}

/// Cleans up old daily note files
///
/// Removes daily note files older than DAILY_NOTE_RETENTION_DAYS (30 days),
/// but always preserves files from the current month.
///
/// # Arguments
/// * `workspace_path` - The workspace directory
///
/// # Returns
/// * `Ok((usize, usize))` - Number of files deleted and bytes freed
/// * `Err(MemoryError)` - If cleanup fails
pub async fn cleanup_old_daily_notes(
    workspace_path: &Path,
) -> Result<(usize, usize), MemoryError> {
    tracing::info!("Starting daily notes cleanup");

    let memory_dir = workspace_path.join("memory");
    if !memory_dir.exists() {
        return Ok((0, 0));
    }

    let cutoff_date = Utc::now().date_naive() - Duration::days(DAILY_NOTE_RETENTION_DAYS);
    let current_month = Utc::now().format("%Y-%m").to_string();

    let mut files_deleted = 0;
    let mut bytes_freed: usize = 0;

    let mut entries = match fs::read_dir(&memory_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            return Err(MemoryError::StorageFailed {
                operation: "read memory directory".to_string(),
                source: e,
            });
        }
    };

    while let Some(entry) = entries.next_entry().await.map_err(|e| MemoryError::StorageFailed {
        operation: "read directory entry".to_string(),
        source: e,
    })? {
        let path = entry.path();
        let file_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Check if filename matches YYYY-MM-DD pattern
        if let Ok(file_date) = NaiveDate::parse_from_str(file_name, "%Y-%m-%d") {
            // Always preserve current month files
            let file_month = file_date.format("%Y-%m").to_string();
            if file_month == current_month {
                tracing::debug!(file = %file_name, "Preserving current month file");
                continue;
            }

            // Check if file is older than retention period
            if file_date < cutoff_date {
                // Get file size before deletion
                match entry.metadata().await {
                    Ok(metadata) => {
                        let size = metadata.len() as usize;

                        match fs::remove_file(&path).await {
                            Ok(_) => {
                                tracing::info!(file = %path.display(), "Deleted old daily note file");
                                files_deleted += 1;
                                bytes_freed += size;
                            }
                            Err(e) => {
                                tracing::error!(file = %path.display(), error = %e, "Failed to delete file");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(file = %path.display(), error = %e, "Failed to read file metadata, skipping");
                    }
                }
            }
        }
    }

    tracing::info!(
        files_deleted = files_deleted,
        bytes_freed = bytes_freed,
        "Daily notes cleanup completed"
    );

    Ok((files_deleted, bytes_freed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
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

    #[tokio::test]
    async fn test_read_recent_days_empty() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        let result = read_recent_days(workspace_path, 7).await;
        assert!(result.is_ok());

        let sections = result.unwrap();
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_read_recent_days_with_files() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        // Create a daily note for today
        create_daily_note(workspace_path, "Test entry 1".to_string(), None::<fn(MemoryEntry)>).await.unwrap();

        let result = read_recent_days(workspace_path, 7).await;
        assert!(result.is_ok());

        let sections = result.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].entries.len(), 1);
        assert_eq!(sections[0].entries[0].content, "Test entry 1");
    }

    #[tokio::test]
    async fn test_read_recent_days_multiple_entries() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        // Create multiple entries for today
        create_daily_note(workspace_path, "First entry".to_string(), None::<fn(MemoryEntry)>).await.unwrap();
        create_daily_note(workspace_path, "Second entry".to_string(), None::<fn(MemoryEntry)>).await.unwrap();

        let result = read_recent_days(workspace_path, 7).await;
        assert!(result.is_ok());

        let sections = result.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].entries.len(), 2);
    }

    #[tokio::test]
    async fn test_parse_daily_note_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("2026-02-16.md");

        // Create test file content
        let content = r#"# Daily Note - 2026-02-16

## 14:30:00 UTC

First entry content

---

## 15:45:00 UTC

Second entry content

---
"#;

        fs::write(&file_path, content).await.unwrap();

        let entries = parse_daily_note_file(&file_path).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "First entry content");
        assert_eq!(entries[1].content, "Second entry content");
    }

    #[tokio::test]
    async fn test_parse_time_string() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        let result = parse_time_string("14:30:00 UTC", date);
        assert!(result.is_some());

        let dt = result.unwrap();
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
        assert_eq!(dt.date_naive(), date);
    }

    #[tokio::test]
    async fn test_cleanup_preserves_current_month() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();
        let memory_dir = workspace_path.join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create a file for today (current month - should be preserved)
        let today = Utc::now().format("%Y-%m-%d");
        let today_file = memory_dir.join(format!("{}.md", today));
        fs::write(&today_file, "Today's content").await.unwrap();

        let (deleted, _) = cleanup_old_daily_notes(workspace_path).await.unwrap();
        assert_eq!(deleted, 0);

        // File should still exist
        assert!(today_file.exists());
    }

    #[tokio::test]
    async fn test_cleanup_deletes_old_files() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();
        let memory_dir = workspace_path.join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create an old file (40 days ago - should be deleted)
        let old_date = (Utc::now() - Duration::days(40)).format("%Y-%m-%d");
        let old_file = memory_dir.join(format!("{}.md", old_date));
        fs::write(&old_file, "Old content").await.unwrap();

        let (deleted, bytes_freed) = cleanup_old_daily_notes(workspace_path).await.unwrap();
        assert_eq!(deleted, 1);
        assert!(bytes_freed > 0);

        // File should be deleted
        assert!(!old_file.exists());
    }

    #[tokio::test]
    async fn test_read_recent_days_chronological_order() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();
        let memory_dir = workspace_path.join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create files for yesterday and today
        let yesterday = (Utc::now() - Duration::days(1)).format("%Y-%m-%d");
        let today = Utc::now().format("%Y-%m-%d");

        let yesterday_file = memory_dir.join(format!("{}.md", yesterday));
        let today_file = memory_dir.join(format!("{}.md", today));

        // Write files with proper format
        fs::write(&yesterday_file, format!("# Daily Note - {}\n\n## 10:00:00 UTC\n\nYesterday entry\n\n---\n", yesterday)).await.unwrap();
        fs::write(&today_file, format!("# Daily Note - {}\n\n## 10:00:00 UTC\n\nToday entry\n\n---\n", today)).await.unwrap();

        let sections = read_recent_days(workspace_path, 7).await.unwrap();
        assert_eq!(sections.len(), 2);

        // Should be sorted chronologically (yesterday first, then today)
        assert!(sections[0].date < sections[1].date);
    }

    #[tokio::test]
    async fn test_read_recent_days_limit_days() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();
        let memory_dir = workspace_path.join("memory");
        fs::create_dir_all(&memory_dir).await.unwrap();

        // Create files for 10 days ago (outside 7-day window)
        let old_date = (Utc::now() - Duration::days(10)).format("%Y-%m-%d");
        let old_file = memory_dir.join(format!("{}.md", old_date));
        fs::write(&old_file, "# Daily Note\n\n## 10:00:00 UTC\n\nOld entry\n\n---\n").await.unwrap();

        // Read only 7 days - should not include the old file
        let sections = read_recent_days(workspace_path, 7).await.unwrap();
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        // No memory directory exists
        let (deleted, bytes_freed) = cleanup_old_daily_notes(workspace_path).await.unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(bytes_freed, 0);
    }

    #[tokio::test]
    async fn test_read_recent_days_missing_files() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        // Only create a file for today
        create_daily_note(workspace_path, "Today entry".to_string(), None::<fn(MemoryEntry)>).await.unwrap();

        // Read 7 days - should only return today's section
        let sections = read_recent_days(workspace_path, 7).await.unwrap();
        assert_eq!(sections.len(), 1);
    }

    #[tokio::test]
    async fn test_parse_daily_note_file_empty() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("2026-02-16.md");

        // Create empty file
        fs::write(&file_path, "").await.unwrap();

        let entries = parse_daily_note_file(&file_path).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_parse_daily_note_file_no_entries() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("2026-02-16.md");

        // Create file with title but no entries
        fs::write(&file_path, "# Daily Note - 2026-02-16\n\n").await.unwrap();

        let entries = parse_daily_note_file(&file_path).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_parse_time_string_invalid() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        assert!(parse_time_string("invalid", date).is_none());
        assert!(parse_time_string("14:30", date).is_none());
        assert!(parse_time_string("14:30:00 EST", date).is_none());
        
        // Edge cases: invalid hours, minutes, seconds
        assert!(parse_time_string("25:00:00 UTC", date).is_none()); // hour > 23
        assert!(parse_time_string("14:60:00 UTC", date).is_none()); // minute > 59
        assert!(parse_time_string("14:30:60 UTC", date).is_none()); // second > 59
        assert!(parse_time_string("-1:30:00 UTC", date).is_none()); // negative hour
    }

    #[tokio::test]
    async fn test_read_recent_days_zero_days() {
        let temp_dir = tempdir().unwrap();
        let workspace_path = temp_dir.path();

        // Create a daily note
        create_daily_note(workspace_path, "Test entry".to_string(), None::<fn(MemoryEntry)>).await.unwrap();

        // Read 0 days should return empty
        let sections = read_recent_days(workspace_path, 0).await.unwrap();
        assert!(sections.is_empty());
    }
}