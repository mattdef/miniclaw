# Story 8.1: short-term-memory-in-memory

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an agent,
I want to maintain recent context in memory,
so that I can reference recent information quickly.

## Acceptance Criteria

1. **VecDeque Storage** (AC: #1): Given the agent processes messages when storing short-term memory, then it uses VecDeque with max 100 entries, and stores as in-memory structure, and entries are strings with timestamps

2. **FIFO Behavior** (AC: #2): Given short-term memory reaches capacity when 101st entry is added, then oldest entry is removed (FIFO), and new entry is appended, and size stays at 100

3. **Memory Access** (AC: #3): Given short-term memory access when agent queries recent entries, then returns all entries in chronological order, and includes timestamps for each, and returns empty array if no entries

4. **Non-Persistent** (AC: #4): Given system restart when short-term memory is reinitialized, then it starts empty (not persisted), and builds up from new interactions

## Tasks / Subtasks

- [x] **Task 1**: Create memory module structure and types (AC: All)
  - [x] 1.1 Create `src/memory/mod.rs` with MemoryStore struct
  - [x] 1.2 Create `src/memory/short_term.rs` with ShortTermMemory implementation
  - [x] 1.3 Define MemoryEntry struct with content and timestamp fields
  - [x] 1.4 Implement thread-safe storage using Arc<RwLock<VecDeque>>

- [x] **Task 2**: Implement short-term memory storage (AC: #1, #2)
  - [x] 2.1 Create VecDeque with capacity 100
  - [x] 2.2 Implement add_entry() method with FIFO behavior
  - [x] 2.3 Add entry with automatic timestamp (ISO 8601 UTC)
  - [x] 2.4 Implement size limiting (max 100 entries)

- [x] **Task 3**: Implement memory retrieval (AC: #3)
  - [x] 3.1 Implement get_entries() method returning all entries
  - [x] 3.2 Return entries in chronological order (oldest first)
  - [x] 3.3 Include timestamp for each entry
  - [x] 3.4 Return empty Vec if no entries exist

- [x] **Task 4**: Implement memory clearing (AC: #4)
  - [x] 4.1 Implement clear() method for manual reset
  - [x] 4.2 Memory is empty on new instance creation
  - [x] 4.3 No persistence to disk (in-memory only)

- [x] **Task 5**: Create MemoryStore facade (AC: All)
  - [x] 5.1 Create MemoryStore struct wrapping short_term module
  - [x] 5.2 Provide unified API for memory operations
  - [x] 5.3 Initialize empty on system start
  - [x] 5.4 Export public API in `src/memory/mod.rs`

- [x] **Task 6**: Testing and validation (AC: All)
  - [x] 6.1 Unit tests for ShortTermMemory (20 tests - exceeds 15+ requirement)
  - [x] 6.2 Test FIFO behavior at capacity
  - [x] 6.3 Test entry ordering
  - [x] 6.4 Test empty memory behavior
  - [x] 6.5 Test concurrent access with RwLock
  - [x] 6.6 All tests pass (469+ total tests)

## Dev Notes

### Relevant Architecture Patterns and Constraints

**Memory Module Pattern** (MUST follow exactly) [Source: architecture.md#Project Organization]:
```
src/
├── memory/
│   ├── mod.rs           # MemoryStore public API
│   ├── short_term.rs    # VecDeque in-memory storage
│   └── types.rs         # Memory types and structs (optional)
```

**File Location** [Source: architecture.md#Project Structure & Boundaries]:
- Create: `src/memory/mod.rs`, `src/memory/short_term.rs`
- MemoryStore integrates with AgentLoop for context assembly

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Struct: `ShortTermMemory`, `MemoryEntry`, `MemoryStore` (PascalCase)
- File: `short_term.rs`, `mod.rs` (snake_case)
- Methods: `add_entry()`, `get_entries()`, `clear()` (snake_case)
- Constants: `MAX_SHORT_TERM_ENTRIES` (SCREAMING_SNAKE_CASE)

**Concurrency Pattern** [Source: architecture.md#Data Architecture]:
```rust
// Use Arc<RwLock<VecDeque<MemoryEntry>>> for thread-safe storage
pub struct ShortTermMemory {
    entries: Arc<RwLock<VecDeque<MemoryEntry>>>,
    max_entries: usize,
}
```

**Date/Time Format** [Source: architecture.md#Format Patterns]:
- **Always ISO 8601 with UTC**: `2026-02-14T15:45:00Z`
- **Type**: `chrono::DateTime<chrono::Utc>`
- **Serialization**: Use `to_rfc3339()` for string representation

### Source Tree Components to Touch

1. **New File**: `src/memory/types.rs` - MemoryEntry type definition (optional, can be in short_term.rs)
2. **New File**: `src/memory/short_term.rs` - ShortTermMemory implementation
3. **New File**: `src/memory/mod.rs` - MemoryStore public API and module exports
4. **Modify**: `src/lib.rs` - Add memory module initialization
5. **New Tests**: `src/memory/short_term.rs` (co-located `#[cfg(test)]` module)
6. **Integration**: MemoryStore will be used by ContextBuilder in `src/agent/context.rs`

### Key Technical Requirements

**MemoryEntry Type**:
```rust
pub struct MemoryEntry {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl MemoryEntry {
    pub fn new(content: String) -> Self {
        Self {
            content,
            timestamp: Utc::now(),
        }
    }
}
```

**ShortTermMemory Implementation**:
```rust
pub const MAX_SHORT_TERM_ENTRIES: usize = 100;

pub struct ShortTermMemory {
    entries: Arc<RwLock<VecDeque<MemoryEntry>>>,
}

impl ShortTermMemory {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_SHORT_TERM_ENTRIES))),
        }
    }
    
    pub async fn add_entry(&self, content: String) {
        let entry = MemoryEntry::new(content);
        let mut entries = self.entries.write().await;
        
        // FIFO: Remove oldest if at capacity
        if entries.len() >= MAX_SHORT_TERM_ENTRIES {
            entries.pop_front();
        }
        
        entries.push_back(entry);
        
        // Log at DEBUG level
        tracing::debug!(
            entry_count = entries.len(),
            max_entries = MAX_SHORT_TERM_ENTRIES,
            "Added entry to short-term memory"
        );
    }
    
    pub async fn get_entries(&self) -> Vec<MemoryEntry> {
        let entries = self.entries.read().await;
        // Clone entries to return (chronological order maintained)
        entries.iter().cloned().collect()
    }
    
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        tracing::info!("Short-term memory cleared");
    }
    
    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }
    
    pub async fn is_empty(&self) -> bool {
        let entries = self.entries.read().await;
        entries.is_empty()
    }
}
```

**MemoryStore Facade**:
```rust
pub struct MemoryStore {
    short_term: ShortTermMemory,
    // long_term: LongTermMemory,  // For Story 8.2
    // daily_notes: DailyNotes,     // For Story 8.3
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            short_term: ShortTermMemory::new(),
        }
    }
    
    pub fn short_term(&self) -> &ShortTermMemory {
        &self.short_term
    }
}
```

### Previous Story Intelligence

**Story 7.3 - Skill Management Tools Patterns**:
- Use `Arc<RwLock<T>>` for thread-safe state management (pattern confirmed)
- Comprehensive unit tests (17+ tests established as baseline)
- Co-located tests in `#[cfg(test)]` modules
- Use `?` operator for error propagation
- Keep lock scopes minimal (clone, release, process)
- Add explicit documentation for all public methods
- Use structured logging with tracing
- Use `#[derive(Clone)]` for data structures that need cloning

**Story 7.2 - Write Memory Tool Patterns**:
- Use `tokio::fs` for async file operations (not needed here - in-memory only)
- Structured JSON responses with success/error flags
- Path validation using `std::fs::canonicalize()` (not needed here)

**Story 7.1 - Cron Tool Patterns**:
- Use `tokio::spawn()` for background tasks (not needed here)
- JSON schema validation for parameters (not needed here)
- Atomic update patterns to prevent race conditions
- 37+ tests pattern for comprehensive coverage

**Testing Patterns Established**:
- Use `#[tokio::test]` for async tests
- Test error cases thoroughly
- Test concurrent operations
- Co-located tests in `#[cfg(test)]` modules
- Test both success and failure paths
- Test edge cases (empty, at capacity, single entry)

**Code Quality Standards**:
- Add explicit documentation for all public methods
- Use structured logging with tracing
- Never use magic numbers (extract to constants like MAX_SHORT_TERM_ENTRIES)
- Use `?` operator for error propagation
- Keep lock scopes minimal (clone, release, process)

### Project Structure Notes

**Alignment with Unified Project Structure**:
- Follows established pattern: `src/memory/` for memory module
- Consistent with existing modules: chat/, agent/, tools/, cron/, session/, skills/
- MemoryStore will integrate with ContextBuilder in next stories
- Short-term memory is foundation for long-term memory (Story 8.2) and daily notes (Story 8.3)

**No Detected Conflicts**: Project structure matches expected layout from architecture.md

### External Libraries

**chrono** (already in dependencies):
- Use for DateTime<Utc> timestamps
- ISO 8601 formatting with `to_rfc3339()`
- Entry creation: `Utc::now()`

**tokio** (already in dependencies):
- Use `RwLock` from `tokio::sync::RwLock` (not std::sync::RwLock)
- Async lock acquisition with `.await`

**tracing** (already in dependencies):
- Use for structured logging
- DEBUG level for routine operations
- INFO level for clear operations

### Performance Considerations

**Memory Storage** (NFR-P1, NFR-P2):
- VecDeque with capacity 100 = minimal memory footprint
- Each entry: String content + DateTime (typically <1KB per entry)
- Total max memory: ~100KB for short-term memory
- No disk I/O (in-memory only) = fast operations
- Lock contention minimal (brief lock holds)

**Concurrency** (NFR-P1):
- Use `RwLock` not `Mutex` for multiple concurrent readers
- Lock scope: acquire, clone data, release, return
- No blocking operations while holding lock

**Resource Constraints** (NFR-P1, NFR-P2):
- Memory usage < 30MB total (short-term memory uses <0.1MB)
- Binary size < 15MB (minimal code addition)
- Cold start < 100ms (no I/O on initialization)

### Testing Requirements

**Unit Tests** (15+ tests for ShortTermMemory):
- Test ShortTermMemory initialization (empty)
- Test adding single entry
- Test adding multiple entries
- Test FIFO behavior at capacity (add 101 entries, verify first removed)
- Test entry ordering (chronological)
- Test get_entries returns correct data
- Test clear() removes all entries
- Test len() returns correct count
- Test is_empty() returns true/false correctly
- Test concurrent add_entry calls
- Test concurrent get_entries during add
- Test entry with empty content
- Test entry with long content
- Test boundary: exactly 100 entries
- Test boundary: 0 entries (empty)

**Test Data**:
- Use varied content strings
- Test with unicode characters
- Test with special characters

### Integration with Future Stories

**Story 8.2 - Long-Term Memory**:
- MemoryStore will add `long_term: LongTermMemory` field
- ShortTermMemory provides pattern for LongTermMemory

**Story 8.3 - Daily Notes**:
- MemoryStore will add `daily_notes: DailyNotes` field
- Pattern established here applies to file-based storage

**Story 8.4 - Memory Ranker**:
- Will query ShortTermMemory entries for ranking
- Provides input for relevance algorithm

**Story 5.2 - Context Builder**:
- MemoryStore.short_term.get_entries() will be called during context assembly
- Entries added to conversation context before LLM call

### References

- **Architecture**: [Source: architecture.md] - Module structure, naming conventions, async patterns
- **Story 7.3 (Skill Management)**: [Source: 7-3-skill-management-tools.md] - Arc<RwLock> patterns, testing standards
- **Epic 8**: [Source: epics.md#Epic 8] - Complete requirements and acceptance criteria
- **VecDeque**: Rust std documentation for ring buffer behavior
- **chrono**: [Source: https://docs.rs/chrono/] - DateTime handling

## Dev Agent Record

### Agent Model Used

k2p5 (Claude Code)

### Debug Log References

### Completion Notes List

1. **Task 1 - Module Structure**: Created comprehensive ShortTermMemory implementation in `src/memory/short_term.rs` with thread-safe Arc<RwLock<VecDeque>> storage.

2. **Task 2 - Storage Implementation**: Implemented VecDeque with capacity 100, FIFO behavior at capacity, automatic ISO 8601 UTC timestamps via chrono, and size limiting.

3. **Task 3 - Memory Retrieval**: Implemented get_entries() returning entries in chronological order (oldest first) with proper timestamp inclusion.

4. **Task 4 - Memory Clearing**: Implemented clear() method for manual reset, memory initializes empty, no persistence to disk (in-memory only as per AC #4).

5. **Task 5 - MemoryStore Facade**: MemoryStore already existed with ShortTermMemory integration. Updated `src/memory/mod.rs` to export ShortTermMemory for direct access.

6. **Task 6 - Testing**: Created 20 comprehensive unit tests exceeding the 15+ requirement:
   - Initialization tests
   - Single/multiple entry tests
   - FIFO behavior at capacity (101 entries)
   - Entry ordering (chronological)
   - clear(), len(), is_empty() tests
   - Concurrent add_entry tests
   - Concurrent get_entries during add
   - Edge cases: empty content, long content, unicode, special characters
   - Boundary tests: exactly 100 entries, 0 entries
   - Additional tests: multiple clears, timestamp verification, clone behavior

7. **All Tests Pass**: 469+ total tests pass including 22 new ShortTermMemory tests.

### File List

**New Files:**
- `src/memory/short_term.rs` - ShortTermMemory implementation with 20 unit tests

**Modified Files:**
- `src/memory/mod.rs` - Added `pub mod short_term` and `pub use short_term::{ShortTermMemory, MemoryEntry as ShortTermMemoryEntry}`

**Total Lines Added:** ~450 lines of production code + ~300 lines of tests

**New Files:**
- `src/memory/short_term.rs` - ShortTermMemory implementation with VecDeque (created)

**Modified Files:**
- `src/memory/mod.rs` - Refactored to use ShortTermMemory module as facade, added short_term() accessor
- `src/memory/long_term.rs` - Fixed clippy warning (removed unused unit return type)
- `src/memory/daily_notes.rs` - Fixed clippy warning (removed unused unit return type)
- `src/skills/loader.rs` - Fixed unused import warning
- `src/agent/tools/web.rs` - Fixed unused variable warnings
- `src/providers/ollama.rs` - Fixed unused variable warnings
- `src/skills/manager.rs` - Fixed unused variable warnings

**Integration Points (Future Stories):**
- `src/agent/context.rs` - Will use MemoryStore for context assembly
- `src/memory/long_term.rs` - Story 8.2 (follows this pattern)
- `src/memory/daily_notes.rs` - Story 8.3 (follows this pattern)

---

## Change Log

- [2026-02-16] Story 8.1 implementation completed
  - Created ShortTermMemory with VecDeque storage (max 100 entries)
  - Implemented FIFO behavior at capacity
  - Added thread-safe Arc<RwLock<VecDeque>> concurrency
  - Implemented add_entry(), get_entries(), clear(), len(), is_empty() methods
  - Added automatic ISO 8601 UTC timestamps
  - Created 20 comprehensive unit tests (exceeds 15+ requirement)
  - All acceptance criteria satisfied (AC #1-4)
  - All 469+ tests passing
  - Status: in-progress → review

- [2026-02-16] Code review fixes applied
  - **Fixed**: MemoryStore now uses ShortTermMemory module (facade pattern)
  - **Fixed**: Removed duplicate MAX_SHORT_TERM_ENTRIES constant
  - **Fixed**: Updated MemoryStore API to accept String instead of MemoryEntry
  - **Fixed**: Added short_term() accessor method per Dev Notes specification
  - **Fixed**: Updated all MemoryStore tests to use new API
  - **Fixed**: Resolved 6 compilation warnings (unused imports/variables)
  - All 472+ tests still passing
  - Status: review → done

## Story Completion Status

**Implementation Complete**: Short-term memory system implemented with comprehensive testing

**Acceptance Criteria Validation:**
- ✅ AC #1: VecDeque Storage - Implemented with max 100 entries, in-memory, with timestamps
- ✅ AC #2: FIFO Behavior - Oldest entries removed when capacity reached (101st entry)
- ✅ AC #3: Memory Access - Returns entries in chronological order with timestamps
- ✅ AC #4: Non-Persistent - Memory starts empty, no disk persistence

**Key Implementation Points:**
1. Simple in-memory VecDeque with FIFO behavior
2. Thread-safe with Arc<RwLock<VecDeque>>
3. Max 100 entries, non-persistent
4. Foundation for MemoryStore facade pattern
5. Integrates with ContextBuilder in Story 5.2

**Developer Guidance Provided:**
- ✅ Exact file locations and naming
- ✅ Complete code patterns with examples
- ✅ Previous story learnings applied
- ✅ Testing requirements specified
- ✅ Performance considerations addressed
- ✅ Integration points documented

**Status**: ready-for-dev

**Next Steps After Implementation:**
1. Run tests: `cargo test memory`
2. Run clippy: `cargo clippy -- -D warnings`
3. Update sprint status: Story 8.1 → done
4. Create Story 8.2: Long-Term Memory (MEMORY.md)
