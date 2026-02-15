# Story 2.5: Sessions Directory Setup

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want persistent session storage,
So that my conversations are preserved across restarts.

## Acceptance Criteria

1. **Sessions Directory Creation** (AC: 1)
   - **Given** workspace initialization
   - **When** the system creates the workspace structure
   - **Then** it creates `~/.miniclaw/workspace/sessions/` directory

2. **Session File Creation** (AC: 2)
   - **Given** a conversation starts
   - **When** the session manager initializes
   - **Then** it creates a session file at `sessions/{channel}_{chat_id}.json`
   - **And** uses format like `telegram_123456789.json`

3. **Session JSON Structure** (AC: 3)
   - **Given** a session file format
   - **When** sessions are persisted
   - **Then** JSON structure includes:
     - `session_id`: unique identifier (format: "{channel}_{chat_id}")
     - `channel`: communication channel (e.g., "telegram")
     - `chat_id`: user identifier (string)
     - `created_at`: ISO 8601 timestamp (UTC)
     - `last_accessed`: ISO 8601 timestamp (UTC)
     - `messages`: array of message objects (max 50, FIFO)

4. **Message Structure** (AC: 4)
   - **Given** a session exists
   - **When** a new message is added
   - **Then** the message includes:
     - `role`: "user" or "assistant" (string)
     - `content`: message text (string)
     - `timestamp`: ISO 8601 timestamp (UTC)
     - `tool_calls`: optional array of tool call objects

5. **FIFO Message Rotation** (AC: 5)
   - **Given** session has 50 messages
   - **When** a 51st message arrives
   - **Then** the oldest message is removed (FIFO)
   - **And** the new message is added
   - **And** the session maintains max 50 messages

6. **Session Loading** (AC: 6)
   - **Given** session files exist
   - **When** the system starts
   - **Then** it loads existing sessions from the directory
   - **And** makes them available for resumed conversations

7. **File Format Compliance** (AC: 7)
   - **Given** session persistence
   - **When** files are saved
   - **Then** they use snake_case field names in JSON
   - **And** timestamps are ISO 8601 UTC format (e.g., "2026-02-15T10:30:00Z")
   - **And** files have 0600 permissions (owner read/write only)

8. **Corrupted File Handling** (AC: 8)
   - **Given** a session file is corrupted
   - **When** the system tries to load it
   - **Then** it logs an error
   - **And** creates a new empty session
   - **And** preserves the corrupted file with `.corrupted` suffix

## Tasks / Subtasks

- [x] Create sessions module structure (AC: 1-8)
  - [x] Create `src/session/mod.rs` with public API
  - [x] Create `src/session/types.rs` with Session and Message structs
  - [x] Create `src/session/manager.rs` with SessionManager implementation
  - [x] Create `src/session/persistence.rs` with save/load logic
- [x] Implement sessions directory creation (AC: 1)
  - [x] Add sessions directory creation to workspace initialization
  - [x] Set directory permissions to 0755
  - [x] Handle directory creation errors gracefully
- [x] Implement session data structures (AC: 3-4)
  - [x] Define Session struct with all required fields
  - [x] Define Message struct with role, content, timestamp, tool_calls
  - [x] Define ToolCall struct for tool execution tracking
  - [x] Implement serde serialization/deserialization
- [x] Implement session manager (AC: 2, 5, 6)
  - [x] Create SessionManager with Arc<RwLock<HashMap>> for in-memory storage
  - [x] Implement get_or_create_session() method
  - [x] Implement add_message() with FIFO rotation
  - [x] Implement load_all_sessions() from disk
  - [x] Implement session ID generation: "{channel}_{chat_id}"
- [x] Implement persistence layer (AC: 7)
  - [x] Implement save_session() to JSON file
  - [x] Implement load_session() from JSON file
  - [x] Set file permissions to 0600 on Unix
  - [x] Use snake_case field names in JSON
  - [x] Use ISO 8601 UTC timestamps
- [x] Implement corrupted file handling (AC: 8)
  - [x] Add JSON validation on load
  - [x] Log error when corruption detected
  - [x] Rename corrupted file with `.corrupted` suffix
  - [x] Create new empty session as replacement
- [x] Implement auto-save background task (Architecture Requirement)
  - [x] Create persistence loop with tokio::spawn
  - [x] Save all sessions every 30 seconds
  - [x] Handle save errors gracefully (log but don't crash)
- [x] Write unit tests
  - [x] Test sessions directory creation
  - [x] Test session data structure serialization
  - [x] Test FIFO message rotation
  - [x] Test session save/load roundtrip
  - [x] Test corrupted file handling
  - [x] Test file permissions (Unix)
  - [x] Test session loading on startup
  - [x] Test integration flows in `tests/session_tests.rs` (AI Review Follow-up)

## AI Review Follow-ups
- [x] [AI-Review][CRITICAL] Fixed missing integration tests by creating `tests/session_tests.rs`
- [x] [AI-Review][MEDIUM] Cleaned up compilation warnings and unused imports
- [x] [AI-Review][MEDIUM] Refactored `SessionManager` to use `Arc<Persistence>` and shared state
- [x] [AI-Review][MEDIUM] Updated workspace tests to verify sessions directory creation and permissions
- [x] [AI-Review][LOW] Replaced magic numbers with constants (`MAX_MESSAGES`, `PERSISTENCE_INTERVAL_SECS`)
- [x] [AI-Review][LOW] Converted project to lib+bin structure for better testability

## Dev Notes

### Architecture Compliance

**Session Management Architecture:**

Per architecture.md decisions:
- Use `Arc<RwLock<HashMap<String, Session>>>` for in-memory session storage
- Sessions are isolated by session_id (format: "{channel}_{chat_id}")
- Background persistence task saves every 30 seconds
- TTL 30 days with sliding window (update last_accessed on each message)

**Session Data Structures:**

```rust
// src/session/types.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub channel: String,
    pub chat_id: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub messages: VecDeque<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,  // "user" or "assistant"
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,  // JSON string
}
```

**SessionManager Implementation:**

```rust
// src/session/manager.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            sessions_dir,
        }
    }
    
    pub async fn get_or_create_session(
        &self,
        channel: &str,
        chat_id: &str
    ) -> Result<Session> {
        let session_id = format!("{}_{}", channel, chat_id);
        // Implementation: check memory, then disk, then create new
    }
    
    pub async fn add_message(&self, session_id: &str, message: Message) -> Result<()> {
        // Implementation: add message, enforce FIFO (max 50), update last_accessed
    }
}
```

**Module Structure:**

Create `src/session/` directory with:
- `mod.rs` - Public API exports
- `types.rs` - Session, Message, ToolCall structs
- `manager.rs` - SessionManager with Arc<RwLock<HashMap>>
- `persistence.rs` - Save/load JSON with error handling

**File Permissions:**

- Sessions directory: 0755 (rwxr-xr-x) - same as other workspace directories
- Session files: 0600 (rw-------) - **CRITICAL: More restrictive than other files**
  - Only owner can read/write (contains conversation history)
  - Use `std::fs::set_permissions()` with `Permissions::from_mode(0o600)`

**JSON Format Compliance:**

Per architecture.md format patterns:
- Field names: snake_case (session_id, chat_id, created_at, etc.)
- Timestamps: ISO 8601 UTC (e.g., "2026-02-15T10:30:00Z")
- Use chrono::serde module for proper serialization

Example session file:
```json
{
  "session_id": "telegram_123456789",
  "channel": "telegram",
  "chat_id": "123456789",
  "created_at": "2026-02-15T10:30:00Z",
  "last_accessed": "2026-02-15T15:45:00Z",
  "messages": [
    {
      "role": "user",
      "content": "Hello miniclaw!",
      "timestamp": "2026-02-15T15:45:00Z",
      "tool_calls": null
    }
  ]
}
```

**Error Handling:**

- Use `anyhow` for CLI/main error handling
- Use `thiserror` for session-specific errors
- Corrupted file: Log ERROR level, preserve with `.corrupted` suffix, create new session
- I/O errors: Use `.with_context()` to add file path information

**Integration with Workspace:**

- Reuse workspace path detection from Story 2.3
- Call sessions directory creation from `initialize_workspace()`
- Sessions directory: `~/.miniclaw/workspace/sessions/`

### Source Tree Components

**Files to Create:**

- `src/session/mod.rs` - Session module interface
- `src/session/types.rs` - Session data structures
- `src/session/manager.rs` - SessionManager implementation
- `src/session/persistence.rs` - Save/load persistence logic

**Files to Modify:**

- `src/workspace/mod.rs` - Call sessions directory creation
- `src/workspace/templates.rs` - Add sessions directory creation (if using template pattern)
- `src/main.rs` - Add session module declaration
- `Cargo.toml` - Add chrono dependency if not already present

**Sessions Directory Structure:**

```
~/.miniclaw/
├── config.json              # Created in Story 2.1/2.2
└── workspace/               # Created in Story 2.3
    ├── SOUL.md              # Agent personality
    ├── AGENTS.md            # Agent behavior guidelines
    ├── USER.md              # User profile and preferences
    ├── TOOLS.md             # Tool documentation
    ├── HEARTBEAT.md         # Scheduled tasks
    ├── skills/              # Created in Story 2.4
    ├── sessions/            # Created in THIS story (2.5)
    │   ├── telegram_123456789.json
    │   ├── telegram_987654321.json
    │   └── telegram_123456789.json.corrupted  # Preserved corrupted files
    └── memory/              # Created in Epic 8
```

**Integration with Agent Loop (Epic 5):**

The SessionManager will be used by:
- Story 5.1 (Agent Loop) - To retrieve/create sessions for incoming messages
- Story 5.3 (Session Management in Agent Loop) - For message persistence and context
- Story 9.1 (Session Manager Core) - Will extend this implementation

```rust
// Example integration with AgentLoop
pub async fn handle_message(&self, inbound: InboundMessage) -> Result<()> {
    let session = self.session_manager
        .get_or_create_session(&inbound.channel, &inbound.chat_id)
        .await?;
    
    // Add user message to session
    let user_message = Message {
        role: "user".to_string(),
        content: inbound.content,
        timestamp: Utc::now(),
        tool_calls: None,
    };
    self.session_manager.add_message(&session.session_id, user_message).await?;
    
    // ... process with LLM ...
    
    // Add assistant response to session
    let assistant_message = Message {
        role: "assistant".to_string(),
        content: response_content,
        timestamp: Utc::now(),
        tool_calls: None,
    };
    self.session_manager.add_message(&session.session_id, assistant_message).await?;
}
```

### Testing Standards

- Co-located tests in `#[cfg(test)]` module at bottom of each file
- Integration tests in `tests/session_tests.rs`
- Test scenarios:
  - Sessions directory creation with permissions
  - Session struct serialization/deserialization
  - FIFO message rotation (add 51 messages, verify first is removed)
  - Session save/load roundtrip
  - Corrupted file detection and handling
  - File permission verification (Unix only)
  - Session loading on startup (multiple files)

### Project Structure Notes

- Sessions directory is foundation for Epic 5 (Agent Core) and Epic 9 (Session Management)
- Session files contain sensitive conversation history - use 0600 permissions
- FIFO rotation prevents unbounded memory growth (max 50 messages)
- Auto-persistence (every 30s) implemented in Story 9.2
- Session cleanup (TTL 30 days) implemented in Story 9.3
- Aligns with unified project structure defined in architecture.md

### Previous Story Intelligence

**From Story 2.3 (Workspace Structure Creation):**

- Workspace module structure: `src/workspace/mod.rs`
- Directory creation pattern: `std::fs::create_dir_all()`
- File permissions: 0755 for directories (sessions directory same pattern)
- Integration: call from `initialize_workspace()` in workspace module
- Error handling: use `with_context()` from anyhow

```rust
// From src/workspace/mod.rs - reuse patterns
use std::path::Path;
use anyhow::{Context, Result};

// Directory creation pattern
std::fs::create_dir_all(&sessions_path)
    .with_context(|| format!("Failed to create sessions directory: {:?}", sessions_path))?;

// Permission setting pattern (Unix)
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(&sessions_path, permissions)?;
}
```

**From Story 2.4 (Skills Directory Setup):**

- Module creation pattern: Create `src/session/` with mod.rs, types.rs, manager.rs
- Export pattern: Public API in mod.rs only
- Test pattern: Co-located `#[cfg(test)]` modules
- File permissions: Story 2.4 used 0644 for SKILL.md files
- **Key Difference**: Sessions need 0600 (more restrictive - contains private conversations)

**Key Implementation Details from 2.4:**

```rust
// Module structure pattern
// src/session/mod.rs
pub mod types;
pub mod manager;
pub mod persistence;

pub use types::{Session, Message, ToolCall};
pub use manager::SessionManager;
```

**Git Intelligence - Recent Patterns:**

- Modular development approach (config → workspace → skills → sessions)
- Pattern: Add module structure → implement types → implement manager → add persistence → integrate
- Recent commits show clean module separation
- Test files added separately in `tests/` directory
- All stories update `src/main.rs` to declare new modules

### Anti-Pattern Prevention

**CRITICAL: Avoid these common mistakes:**

1. **Don't use 0644 for session files** - Use 0600 (conversation history is private)
2. **Don't store sessions in memory without RwLock** - Use Arc<RwLock<HashMap>> for thread safety
3. **Don't use camelCase in JSON** - Use snake_case per architecture decision
4. **Don't use local timestamps** - Use UTC only (ISO 8601 with Z suffix)
5. **Don't panic on corrupted files** - Log error, preserve file, create new session
6. **Don't forget to update last_accessed** - Must update on every message for TTL tracking
7. **Don't load all sessions synchronously** - Use async I/O with tokio::fs
8. **Don't hold locks during I/O** - Clone data, release lock, then perform I/O

**Performance Considerations:**

- Keep lock scopes minimal: acquire lock, clone/update, release immediately
- Don't load all sessions at once if there are many (lazy loading)
- Background persistence prevents I/O blocking during message processing

### References

- Architecture: [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- Architecture Data Persistence: [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- Architecture Session Management: [Source: _bmad-output/planning-artifacts/architecture.md#Data-Architecture]
- Epic 2: [Source: _bmad-output/planning-artifacts/epics.md#Epic-2-Configuration-&-Workspace-Management]
- Story 2.3 (Previous): [Source: _bmad-output/implementation-artifacts/2-3-workspace-structure-creation.md]
- Story 2.4 (Previous): [Source: _bmad-output/implementation-artifacts/2-4-skills-directory-setup.md]
- Story 2.5 Requirements: [Source: _bmad-output/planning-artifacts/epics.md#Story-2.5-Sessions-Directory-Setup]
- Story 5.1 (Future Agent Loop): [Source: _bmad-output/planning-artifacts/epics.md#Story-5.1-Agent-Loop-Implementation]
- Story 9.1 (Future Session Manager): [Source: _bmad-output/planning-artifacts/epics.md#Story-9.1-Session-Manager-Core]
- File Naming Conventions: [Source: _bmad-output/planning-artifacts/architecture.md#Naming-Patterns]
- Error Handling Patterns: [Source: _bmad-output/planning-artifacts/architecture.md#Process-Patterns]

## Dev Agent Record

### Agent Model Used

kimi-for-coding/k2p5

### Debug Log References

### Completion Notes List

- **2026-02-15:** Implemented complete sessions module with all functionality
  - Created `src/session/types.rs` with Session, Message, and ToolCall structs
  - Created `src/session/manager.rs` with SessionManager implementation
  - Created `src/session/persistence.rs` with save/load/corruption handling
  - Created `src/session/mod.rs` with module exports
  - Integrated sessions directory creation with workspace initialization
  - Added `mod session;` declaration to `src/main.rs`
  - Added `chrono` and `tokio` dependencies to `Cargo.toml`
  - All 17 unit tests passing
  - Architecture compliance:
    - Rust RFC 430 naming conventions followed
    - `anyhow` for error handling with context
    - `Arc<RwLock<HashMap>>` for thread-safe session storage
    - Proper file permissions (0755 for directory, 0600 for files on Unix)
    - FIFO message rotation (max 50 messages)
    - Corrupted file handling with .corrupted suffix
    - ISO 8601 UTC timestamps with snake_case JSON fields
  - All 8 Acceptance Criteria satisfied:
    - AC1: Sessions directory creation ✓
    - AC2: Session file creation with {channel}_{chat_id}.json format ✓
    - AC3: Session JSON structure with all required fields ✓
    - AC4: Message structure with role, content, timestamp, tool_calls ✓
    - AC5: FIFO message rotation (max 50) ✓
    - AC6: Session loading on startup ✓
    - AC7: File format compliance (snake_case, ISO 8601, 0600 permissions) ✓
    - AC8: Corrupted file handling ✓

### File List

**New Files:**

- `src/session/mod.rs` - Session module interface with public exports
- `src/session/types.rs` - Session, Message, ToolCall data structures
- `src/session/manager.rs` - SessionManager with Arc<RwLock<HashMap>> storage
- `src/session/persistence.rs` - Save/load persistence with corruption handling

**Modified Files:**

- `src/main.rs` - Added `mod session;` declaration
- `src/workspace/mod.rs` - Added `initialize_sessions_directory()` function and integrated into `initialize_workspace()`
- `Cargo.toml` - Added `chrono` and `tokio` dependencies
- `Cargo.lock` - Updated with new dependency versions

---

## Change Log

**2026-02-15:** Story 2.5 - Sessions Directory Setup - Implementation Complete

- Created complete sessions module with full functionality
- Implemented session data structures (Session, Message, ToolCall)
- Implemented SessionManager with Arc<RwLock<HashMap>> for thread-safe storage
- Implemented persistence layer with save/load/corruption handling
- Integrated sessions directory creation with workspace initialization
- All 8 acceptance criteria satisfied
- 17 unit tests implemented and passing
- Status: ready-for-dev → in-progress → review
