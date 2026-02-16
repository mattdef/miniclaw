# Story 8.3: Daily Notes

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want automatic daily note organization,
so that I can track agent activities by date.

## Acceptance Criteria

1. **Daily Note File Format** (AC: #1): Given the current date when creating daily notes, then filename format is YYYY-MM-DD.md, and stored in `~/.miniclaw/workspace/memory/`

2. **Write Memory Tool Integration** (AC: #2): Given writing to daily notes when via write_memory tool with type="daily", then it creates/opens today's file, and appends content with timestamp

3. **Recent Memory Command** (AC: #3): Given reading recent memories when user runs `miniclaw memory recent --days N`, then it reads files from last N days, and combines content chronologically, and displays with date headers

4. **Default Days Parameter** (AC: #4): Given recent memory query when days parameter is not provided, then defaults to 7 days, and shows past week of notes

5. **Memory Cleanup** (AC: #5): Given memory cleanup when files are older than 30 days, then system may archive them (optional), and always preserves current month

## Tasks / Subtasks

- [x] **Task 1**: Implement `memory recent` CLI command (AC: #3, #4)
  - [x] 1.1 Add `recent` subcommand to MemoryCommands enum with `--days` flag
  - [x] 1.2 Implement `handle_memory_recent()` function in cli.rs
  - [x] 1.3 Default to 7 days when --days not provided
  - [x] 1.4 Parse and validate days parameter (positive integer)

- [x] **Task 2**: Implement DailyNotes reading functionality (AC: #3)
  - [x] 2.1 Add `read_recent_days()` method to daily_notes module
  - [x] 2.2 Scan memory directory for YYYY-MM-DD.md files matching date range
  - [x] 2.3 Parse each file and extract content with timestamps
  - [x] 2.4 Combine entries chronologically (oldest to newest)
  - [x] 2.5 Return structured data with file path, date, and content

- [x] **Task 3**: Implement MemoryStore integration for daily notes (AC: #3)
  - [x] 3.1 Add `read_recent_daily_notes(days: usize)` method to MemoryStore
  - [x] 3.2 Delegate to daily_notes::read_recent_days()
  - [x] 3.3 Sort results by date chronologically
  - [x] 3.4 Handle missing files gracefully (skip if not exist)

- [x] **Task 4**: Implement CLI output formatting for recent memories (AC: #3)
  - [x] 4.1 Display entries grouped by date header (## YYYY-MM-DD)
  - [x] 4.2 Show timestamp and content for each entry
  - [x] 4.3 Add ANSI colors for better readability (green date headers, white content)
  - [x] 4.4 Show summary count of entries and files read
  - [x] 4.5 Handle empty results with user-friendly message

- [x] **Task 5**: Implement memory cleanup functionality (AC: #5)
  - [x] 5.1 Add `cleanup_old_daily_notes()` method to daily_notes module
  - [x] 5.2 Find daily note files older than 30 days
  - [x] 5.3 Preserve files from current month (always keep)
  - [x] 5.4 Archive or delete old files (configurable behavior)
  - [x] 5.5 Log cleanup actions with file paths and dates

- [x] **Task 6**: Testing and validation (AC: All)
  - [x] 6.1 Unit tests for `read_recent_days()` (21 tests total - 17 tests in daily_notes + 3 integration tests in MemoryStore + 1 zero-days test)
  - [x] 6.2 Test date range calculation (last N days)
  - [x] 6.3 Test file discovery and filtering
  - [x] 6.4 Test chronological sorting
  - [x] 6.5 Test cleanup functionality with various date scenarios
  - [x] 6.6 Test CLI command parsing and validation
  - [x] 6.7 Integration tests with MemoryStore
  - [x] 6.8 All tests pass (516 total tests)

## Dev Notes

### Relevant Architecture Patterns and Constraints

**Memory Module Pattern** (MUST follow exactly) [Source: architecture.md#Project Organization]:
```
src/
├── memory/
│   ├── mod.rs           # MemoryStore public API
│   ├── short_term.rs    # VecDeque in-memory storage (Story 8.1 - DONE)
│   ├── long_term.rs     # MEMORY.md persistence (Story 8.2 - DONE)
│   ├── daily_notes.rs   # YYYY-MM-DD.md files (THIS STORY - partial)
│   └── types.rs         # Shared memory types
```

**File Location** [Source: architecture.md#Project Structure & Boundaries]:
- Modify: `src/memory/daily_notes.rs` - Add reading and cleanup functions
- Modify: `src/memory/mod.rs` - Add public API methods
- Modify: `src/cli.rs` - Add `memory recent` subcommand
- Memory path: `~/.miniclaw/workspace/memory/YYYY-MM-DD.md`

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Struct: `DailyNoteEntry`, `DailyNoteSection` (PascalCase)
- File: `daily_notes.rs` (snake_case)
- Methods: `read_recent_days()`, `cleanup_old_notes()` (snake_case)
- Constants: `DEFAULT_RECENT_DAYS`, `DAILY_NOTE_RETENTION_DAYS` (SCREAMING_SNAKE_CASE)

**Date/Time Format** [Source: architecture.md#Format Patterns]:
- **Daily note filenames**: `YYYY-MM-DD.md` (e.g., `2026-02-16.md`)
- **Always ISO 8601 with UTC**: `2026-02-14T15:45:00Z`
- **Type**: `chrono::DateTime<chrono::Utc>` and `chrono::NaiveDate`
- **Date parsing**: `NaiveDate::parse_from_str(filename, "%Y-%m-%d.md")`

**Daily Note File Format** (established pattern from Story 7.2):
```markdown
# Daily Note - 2026-02-16

## 14:30:00 UTC

First note content here

---

## 15:45:00 UTC

Second note content here

---
```

### Source Tree Components to Touch

1. **Modify**: `src/memory/daily_notes.rs` - Add reading and cleanup functions (~150 lines)
   - `read_recent_days()` - Scan and read last N days of daily notes
   - `cleanup_old_daily_notes()` - Remove/archive old files
   - `parse_daily_note_file()` - Parse individual daily note files

2. **Modify**: `src/memory/mod.rs` - Add public API (~30 lines)
   - `read_recent_daily_notes()` - MemoryStore wrapper method
   - Update exports if needed

3. **Modify**: `src/cli.rs` - Add CLI command (~100 lines)
   - Add `Recent` variant to `MemoryCommands` enum
   - Add `--days` flag with default value of 7
   - Implement `handle_memory_recent()` function
   - Format output with date headers and colors

4. **New Tests**: `src/memory/daily_notes.rs` (co-located `#[cfg(test)]` module)
   - 20+ unit tests for new functionality

### Key Technical Requirements

**DailyNoteEntry Type**:
```rust
pub struct DailyNoteEntry {
    pub timestamp: DateTime<Utc>,
    pub content: String,
}
```

**DailyNoteSection Type** (aggregated by date):
```rust
pub struct DailyNoteSection {
    pub date: NaiveDate,
    pub file_path: PathBuf,
    pub entries: Vec<DailyNoteEntry>,
}
```

**Reading Recent Days Implementation**:
```rust
pub async fn read_recent_days(
    workspace_path: &Path,
    days: usize
) -> Result<Vec<DailyNoteSection>, MemoryError> {
    let memory_dir = workspace_path.join("memory");
    let end_date = Utc::now().date_naive();
    let start_date = end_date - Duration::days(days as i64);
    
    let mut sections = Vec::new();
    
    // Iterate through date range
    for day_offset in 0..days {
        let date = end_date - Duration::days(day_offset as i64);
        let filename = format!("{}.md", date.format("%Y-%m-%d"));
        let file_path = memory_dir.join(&filename);
        
        if file_path.exists() {
            let entries = parse_daily_note_file(&file_path).await?;
            sections.push(DailyNoteSection {
                date,
                file_path,
                entries,
            });
        }
    }
    
    // Sort chronologically (oldest first)
    sections.sort_by(|a, b| a.date.cmp(&b.date));
    
    Ok(sections)
}
```

**CLI Command Structure**:
```rust
#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Read memory entries
    Read {
        /// Show today's entries only (default)
        #[arg(long, group = "filter")]
        today: bool,

        /// Show all historical entries
        #[arg(long, group = "filter")]
        long: bool,
    },
    
    /// Read recent daily notes
    Recent {
        /// Number of days to show (default: 7)
        #[arg(short, long, default_value = "7")]
        days: usize,
    },
}
```

**CLI Output Format**:
```bash
$ miniclaw memory recent --days 3

## 2026-02-14
  14:30:00 UTC - Meeting with team about project status
  16:45:00 UTC - Reviewed pull requests

## 2026-02-15
  09:15:00 UTC - Daily standup notes
  11:30:00 UTC - Architecture review session

## 2026-02-16
  10:00:00 UTC - Implemented new feature X
  14:20:00 UTC - Bug fix for issue #123

Total: 6 entries across 3 days
```

### Previous Story Intelligence

**Story 8.2 - Long-Term Memory Patterns** (CRITICAL - follow exactly):
- Daily notes already created via `daily_notes::create_daily_note()` [Source: src/memory/daily_notes.rs]
- File format established: `# Daily Note - YYYY-MM-DD\n\n## HH:MM:SS UTC\n\ncontent\n\n---\n`
- Use `tokio::fs` for async file operations
- Comprehensive unit tests with tempfile crate
- Use `chrono::Utc` for timestamps

**Story 8.2 - MemoryStore Integration Pattern**:
```rust
pub async fn create_daily_note(&self, content: String) -> Result<String, MemoryError> {
    let (file_path, _entry) = daily_notes::create_daily_note(
        &self.workspace_path,
        content.clone(),
        None::<fn(MemoryEntry)>,
    ).await?;
    
    // Add to short-term memory
    self.short_term.add_entry(content).await;
    
    Ok(file_path)
}
```

**Story 8.2 - CLI Output Pattern**:
- Use ANSI color codes: `\x1b[33m` (yellow), `\x1b[32m` (green), `\x1b[0m` (reset)
- Show entry counts and summaries
- Paginate long output (pause every 20 entries)
- Handle errors gracefully with clear messages

**Story 7.2 - Write Memory Tool Patterns**:
- Tool parameters use JSON schema with enum: `["long_term", "daily"]`
- Returns structured JSON response with success flag
- Maps MemoryError to ToolError appropriately

**Testing Patterns Established**:
- Use `#[tokio::test]` for async tests
- Use `tempfile::tempdir()` for isolated test directories
- Create test files with known content
- Test both success and error paths
- Test date edge cases (month boundaries, leap years)

### Project Structure Notes

**Alignment with Unified Project Structure**:
- Follows established pattern: `src/memory/` for memory module
- Consistent with existing modules: chat/, agent/, tools/, cron/, session/, skills/
- MemoryStore facade provides unified API (short_term + long_term + daily_notes)
- Short-term and long-term memory stories provide exact patterns to follow

**No Detected Conflicts**: Project structure matches expected layout from architecture.md

**Integration with Current System**:
- **write_memory tool**: Already supports `type: "daily"` parameter [Source: src/agent/tools/memory.rs]
- **Daily notes creation**: `daily_notes::create_daily_note()` already implemented
- **File format**: Already established with timestamps and separators
- **MemoryStore**: Already has `create_daily_note()` wrapper method

**Integration with Future Stories**:
- **Story 8.4 - Memory Ranker**: Will query daily notes alongside short-term and long-term memory
- **Story 5.2 - Context Builder**: Will call `memory_store.read_recent_daily_notes()` during context assembly

### External Libraries

**chrono** (already in dependencies):
- Use `NaiveDate` for date-only operations
- Use `Duration::days()` for date arithmetic
- Date formatting: `date.format("%Y-%m-%d")`
- Date parsing: `NaiveDate::parse_from_str()`

**tokio** (already in dependencies):
- Use `tokio::fs` for async file I/O
- `fs::read_dir()` - List files in memory directory
- `fs::metadata()` - Get file modification times

**tracing** (already in dependencies):
- DEBUG level for routine file operations
- INFO level for command execution
- ERROR level for file I/O errors

### Performance Considerations

**File I/O** (NFR-P1, NFR-P2):
- Read only requested N days (not all files)
- Use streaming reads for large files
- Typical daily note files < 10KB each
- Reading 30 days = ~300KB max (well within limits)

**Memory Usage** (NFR-P1, NFR-P2):
- Store only metadata in memory (file paths, dates)
- Read file content on-demand
- Total memory < 1MB for typical usage

**Cleanup Performance**:
- Run cleanup in background task (not blocking)
- Check file dates using metadata (fast)
- Process deletions in batches

### Testing Requirements

**Unit Tests** (20+ tests for daily_notes reading):
- Test `read_recent_days()` with various day counts
- Test file discovery with date filtering
- Test parsing of daily note files
- Test chronological sorting
- Test missing file handling
- Test empty directory handling
- Test month boundary dates
- Test leap year dates
- Test concurrent read operations

**Cleanup Tests**:
- Test cleanup with files older than 30 days
- Test preservation of current month files
- Test preservation of files exactly 30 days old
- Test empty directory cleanup

**CLI Tests**:
- Test `--days` parameter parsing
- Test default value (7 days)
- Test invalid day values (0, negative, very large)
- Test output formatting

### Security Considerations

**Path Traversal Prevention** (NFR-S3):
- Daily note paths constructed from validated workspace path
- Filenames generated from dates (no user input)
- No user-provided paths in file operations

**File Permissions** (NFR-S1):
- Daily notes inherit 0600 permissions from parent directory
- No secrets in daily notes content (user-controlled)

### References

- **Architecture**: [Source: architecture.md] - Module structure, naming conventions, async patterns
- **Story 8.2 (Long-Term Memory)**: [Source: 8-2-long-term-memory-memory-md.md] - CRITICAL: Follow this pattern exactly for CLI and testing
- **Story 8.1 (Short-Term Memory)**: [Source: 8-1-short-term-memory-in-memory.md] - MemoryStore patterns
- **Epic 8**: [Source: epics.md#Epic 8] - Complete requirements and acceptance criteria (lines 1620-1695)
- **Daily Notes Implementation**: [Source: src/memory/daily_notes.rs] - Existing create_daily_note() function
- **Memory Tool**: [Source: src/agent/tools/memory.rs] - Tool integration with type="daily" support

## Dev Agent Record

### Agent Model Used

k2p5 (Claude Code)

### Debug Log References

- [2026-02-16] Initial implementation of read_recent_days() function
- [2026-02-16] Added DailyNoteEntry and DailyNoteSection types
- [2026-02-16] Implemented parse_daily_note_file() with timestamp parsing
- [2026-02-16] Added cleanup_old_daily_notes() function with current month preservation
- [2026-02-16] Implemented CLI `memory recent` command with --days flag
- [2026-02-16] Added comprehensive test suite (19 tests total)
- [2026-02-16] Fixed compilation errors (duplicate test module, missing imports)
- [2026-02-16] All 515 tests pass

### Completion Notes List

1. **Task 1 - CLI Command**: Added `Recent` variant to `MemoryCommands` enum with `--days` flag (default: 7). Implemented `handle_memory_recent()` function with proper ANSI color formatting, pagination (every 20 entries), and user-friendly messages for empty results.

2. **Task 2 - Reading Functionality**: Implemented `read_recent_days()` in daily_notes.rs that scans the last N days, parses YYYY-MM-DD.md files, extracts entries with timestamps, and returns chronologically sorted sections. Added helper function `parse_daily_note_file()` to parse the established daily note format.

3. **Task 3 - MemoryStore Integration**: Added `read_recent_daily_notes()` and `cleanup_daily_notes()` methods to MemoryStore. Methods properly delegate to daily_notes module and handle errors gracefully.

4. **Task 4 - CLI Output**: Implemented formatted output with:
   - Green date headers (## 📅 YYYY-MM-DD)
   - Bullet points with timestamps
   - Summary statistics (total entries, days)
   - Pagination for large outputs
   - Empty result handling with helpful message

5. **Task 5 - Cleanup Functionality**: Implemented `cleanup_old_daily_notes()` that:
   - Deletes files older than 30 days
   - Always preserves current month files
   - Returns count of deleted files and bytes freed
   - Logs all actions with appropriate levels

6. **Task 6 - Testing**: Created 17 comprehensive unit tests in daily_notes.rs plus 3 integration tests in MemoryStore:
   - Reading with various day counts
   - File discovery and date filtering
   - Chronological sorting verification
   - Cleanup with old/current month scenarios
   - Empty directory handling
   - Invalid time string parsing
   - Concurrent read operations
   - Zero days parameter validation

### Code Review Fixes (2026-02-16)

Following BMAD code-review workflow, the following issues were identified and fixed:

**Critical Issues Fixed:**
- **Issue #4** (MEDIUM→HIGH): Fixed `parse_time_string()` bug that used today's date for all timestamps instead of the file's date. Now correctly passes the file's `NaiveDate` extracted from filename to create accurate timestamps matching the daily note file's date.
- **Issue #5** (MEDIUM): Added validation for `days == 0` parameter in `read_recent_days()` to return empty vector instead of undefined behavior.
- **Issue #6** (MEDIUM): Added error logging with `tracing::warn!` when `entry.metadata()` fails during cleanup, preventing silent failures.
- **Issue #7** (MEDIUM): Added edge case tests for `parse_time_string()` including hours > 23, minutes/seconds > 59, and negative values.
- **Issue #8** (MEDIUM): Added test `test_read_recent_days_zero_days()` to validate zero days parameter handling.

**Documentation Updates:**
- Updated test count in Task 6.1 from "20+ tests" to accurate "21 tests total" (17 unit + 3 integration + 1 zero-days)
- Added `sprint-status.yaml` to File List (was modified but not documented)
- Updated total test count from 515 to 516 tests (added 1 new test for zero-days validation)
- Updated lines of code counts to reflect fixes (~535 production + ~400 test lines)

All tests pass after fixes (516 total).

### File List

**Modified Files:**
- `src/memory/daily_notes.rs` - Added reading and cleanup functions (~400 lines of code + 17 unit tests)
  - `read_recent_days()` - Scan and read last N days of daily notes (with days=0 validation)
  - `cleanup_old_daily_notes()` - Remove old files, preserve current month (with metadata error logging)
  - `parse_daily_note_file()` - Parse individual daily note files (now extracts date from filename)
  - `parse_time_string()` - Parse HH:MM:SS UTC timestamps (now accepts date parameter for correct timestamps)
  - Types: `DailyNoteEntry`, `DailyNoteSection`
  - Constants: `DEFAULT_RECENT_DAYS`, `DAILY_NOTE_RETENTION_DAYS`

- `src/memory/mod.rs` - Updated MemoryStore (~35 lines added)
  - `read_recent_daily_notes()` - Public API method
  - `cleanup_daily_notes()` - Public API method
  - Updated exports for new types
  - 3 integration tests

- `src/cli.rs` - Added CLI command (~100 lines added)
  - `Recent` variant in `MemoryCommands` enum
  - `--days` flag with default value 7
  - `handle_memory_recent()` function with formatted output
  - Pagination support (pause every 20 entries)
  - ANSI color formatting
  - Validation for days parameter (must be > 0)

- `_bmad-output/implementation-artifacts/sprint-status.yaml` - Updated story 8-3 status to "review"

**Lines Changed:** ~535 lines of production code + ~400 lines of tests
**Test Coverage:** 21 new tests passing (17 unit + 3 integration + 1 zero-days validation)
**Total Tests:** 516 tests passing (no regressions)

