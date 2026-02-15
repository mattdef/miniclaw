# Story 5.3: Session Management in Agent Loop

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want my conversation history preserved,
so that the agent remembers context across messages.

## Acceptance Criteria

1. **Session Creation**: Given a conversation starts, when the first message arrives, then a new session is created, and session_id is generated: "{channel}_{chat_id}", and created_at timestamp is recorded

2. **Session Update**: Given a session exists, when a message is processed, then user message is added to session, and assistant response is added to session, and last_accessed timestamp is updated

3. **FIFO Management**: Given session reaches 50 messages, when a new message arrives, then the oldest message is removed, and the new message is appended, and session maintains FIFO order

4. **Tool Interaction Storage**: Given a conversation has tool interactions, when tools are called, then tool_calls are stored in assistant message, and tool results are stored as separate messages, and the interaction flow is preserved

5. **Auto-Persistence**: Given session persistence is configured, when 30 seconds elapse, then sessions are automatically saved to disk, and saved to `~/.miniclaw/workspace/sessions/`, and save happens in background (non-blocking)

6. **Session Recovery**: Given the system restarts, when it loads previous sessions, then it reads all session files from disk, and makes them available for resumed conversations, and maintains conversation continuity

## Tasks / Subtasks

- [x] **Task 1**: Integrate SessionManager with Agent Loop (AC: #1, #2)
  - [x] 1.1 Initialize SessionManager in AgentLoop::new() with workspace path
  - [x] 1.2 Get or create session on first message using channel + chat_id
  - [x] 1.3 Add user message to session before processing
  - [x] 1.4 Add assistant response to session after LLM completion
  - [x] 1.5 Update last_accessed timestamp on every interaction

- [x] **Task 2**: Implement session lifecycle hooks (AC: #2, #3)
  - [x] 2.1 Create SessionMessage type with role, content, timestamp, tool_calls
  - [x] 2.2 Enforce 50 message limit with VecDeque FIFO behavior
  - [x] 2.3 Add tool_calls field to assistant messages when tools are invoked
  - [x] 2.4 Store tool results as separate messages with tool_result role
  - [x] 2.5 Ensure atomic session updates within AgentLoop iteration

- [x] **Task 3**: Connect persistence to Agent Loop lifecycle (AC: #5)
  - [x] 3.1 Spawn background persistence task in gateway mode
  - [x] 3.2 Configure 30-second auto-save interval via tokio::time::interval
  - [x] 3.3 Trigger save on graceful shutdown (SIGTERM)
  - [x] 3.4 Ensure non-blocking save operations (spawn_blocking for disk I/O)
  - [x] 3.5 Handle persistence errors without crashing agent loop

- [x] **Task 4**: Implement session loading on startup (AC: #6)
  - [x] 4.1 Load all session files from ~/.miniclaw/workspace/sessions/ on startup
  - [x] 4.2 Parse JSON session files with proper error handling
  - [x] 4.3 Validate session structure and skip corrupted files
  - [x] 4.4 Populate SessionManager HashMap with loaded sessions
  - [x] 4.5 Log session recovery statistics on startup

- [x] **Task 5**: Integration with ContextBuilder (AC: #2, #4)
  - [x] 5.1 Expose get_session_history() method from SessionManager
  - [x] 5.2 Pass session messages to ContextBuilder for history layer
  - [x] 5.3 Format tool_calls in messages for LLM context
  - [x] 5.4 Ensure session updates visible to ContextBuilder on next iteration

- [x] **Task 6**: Testing and validation
  - [x] 6.1 Unit tests for session lifecycle (create, update, FIFO)
  - [x] 6.2 Unit tests for tool interaction storage
  - [x] 6.3 Integration tests for persistence timing
  - [x] 6.4 Integration tests for session recovery on restart
  - [x] 6.5 Test concurrent access to same session

## Dev Notes

### Relevant Architecture Patterns and Constraints

**Session Architecture** (MUST follow exactly) [Source: architecture.md#Data Architecture]:
- **Concurrency**: Arc<RwLock<HashMap<String, Session>>> for thread-safe session storage
- **Persistence**: Auto-save every 30 seconds via tokio::spawn background task
- **Rotation**: TTL 30 days with sliding window for inactive sessions
- **File Format**: JSON with snake_case fields, ISO 8601 UTC timestamps
- **Permissions**: 0600 on session files

**Session JSON Structure** [Source: architecture.md#Implementation Patterns & Consistency Rules]:
```json
{
  "session_id": "telegram_123456789",
  "channel": "telegram",
  "chat_id": "123456789",
  "created_at": "2026-02-14T10:30:00Z",
  "last_accessed": "2026-02-14T15:45:00Z",
  "messages": [
    {
      "role": "user",
      "content": "Hello miniclaw!",
      "timestamp": "2026-02-14T15:45:00Z",
      "tool_calls": null
    },
    {
      "role": "assistant",
      "content": "Hello! How can I help you?",
      "timestamp": "2026-02-14T15:45:05Z",
      "tool_calls": [...]
    },
    {
      "role": "tool_result",
      "content": "{\"result\": \"...\"}",
      "timestamp": "2026-02-14T15:45:10Z",
      "tool_calls": null
    }
  ]
}
```

**Integration Points**:
- Agent Loop creates/updates sessions during message processing
- ContextBuilder reads session history via SessionManager
- Persistence task runs independently, saving dirty sessions
- Session files stored in `~/.miniclaw/workspace/sessions/{channel}_{chat_id}.json`

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Structs: PascalCase (`SessionManager`, `Session`)
- Methods: snake_case (`get_or_create_session()`, `add_message()`)
- Fields: snake_case (`session_id`, `last_accessed`, `chat_id`)
- Constants: SCREAMING_SNAKE_CASE (`MAX_MESSAGES`, `PERSISTENCE_INTERVAL_SECS`)

### Source Tree Components to Touch

**Files to Create/Modify**:
1. `src/session/mod.rs` - Extend SessionManager with persistence integration
2. `src/session/types.rs` - Define SessionMessage, Session structs with tool_calls support
3. `src/agent/loop.rs` - Integrate session management into AgentLoop
4. `src/agent/mod.rs` - Update AgentLoop constructor to accept SessionManager

**Dependencies Required**:
- tokio::time::interval for persistence loop
- tokio::fs for async file operations
- chrono::serde for timestamp serialization
- Arc<RwLock<...>> for concurrent session access

### Testing Standards Summary

**Test Coverage Required**:
- Session lifecycle: create, get_or_create, update, FIFO eviction
- Tool interaction: store tool_calls, retrieve tool results
- Persistence: timing, error handling, concurrent saves
- Recovery: load on startup, corrupted file handling
- Integration: AgentLoop + SessionManager + ContextBuilder

**Test Patterns**:
- Use `#[tokio::test]` for async tests
- Use temp directories for session file tests
- Mock filesystem for persistence error cases
- Use `tokio::time::pause()` for controlling time in persistence tests

## Project Structure Notes

### Alignment with Unified Project Structure

**Expected Structure** (from architecture.md):
```
src/session/
├── mod.rs              # SessionManager public API - EXTEND
├── manager.rs          # Session management logic - MAYBE
├── persistence.rs      # Auto-save every 30s - CREATE
├── cleanup.rs          # TTL cleanup task - STUB
└── types.rs            # Session struct & types - EXTEND
```

**Integration with Agent Module**:
```
src/agent/
├── mod.rs              # Update AgentLoop constructor
├── loop.rs             # Integrate session lifecycle hooks
├── context.rs          # Use session.get_history() - ALREADY DONE (5.2)
└── types.rs            # Agent types & state
```

### Detected Conflicts or Variances

**None expected** - This story builds directly on existing architecture decisions:
- SessionManager trait already defined in Story 2.5 (sessions directory setup)
- AgentLoop trait already exists from Story 5.1
- ContextBuilder already expects session history from Story 5.2
- All file paths follow established patterns

## References

### Technical Details with Source Paths

**Session Management Architecture**:
- [Source: architecture.md#Data Architecture] - Arc<RwLock<HashMap>> pattern, 30s persistence, TTL 30j
- [Source: architecture.md#Implementation Patterns & Consistency Rules] - JSON format, snake_case, ISO 8601 UTC
- [Source: architecture.md#Project Structure & Boundaries] - SessionManager component boundaries

**Session File Format**:
- [Source: architecture.md#Format Patterns] - Complete JSON structure example
- [Source: epics.md#Story 2.5] - Session file naming convention: {channel}_{chat_id}.json

**Agent Loop Integration**:
- [Source: epics.md#Story 5.1] - Agent Loop phases and iteration counter
- [Source: epics.md#Story 5.2] - ContextBuilder integration with session history
- [Source: epics.md#Story 5.3] - This story's acceptance criteria

**NFRs Applicable**:
- [Source: architecture.md#Non-Functional Requirements Coverage] - NFR-R4 (persistence every 30s)
- [Source: architecture.md#Security Boundaries] - NFR-S1 (file permissions 0600)

### Previous Story Intelligence

**From Story 5.1 (Agent Loop)** [Source: 5-1-agent-loop-implementation.md]:
- Agent Loop structure: Receive→Context→LLM→Tools→Reply cycle
- Iteration counter with 200 max limit
- Error handling: Tool failures captured and continue loop

**From Story 5.2 (Context Builder)** [Source: 5-2-context-builder.md]:
- ContextBuilder retrieves session history (max 50 messages)
- History layer: up to 50 most recent messages from session
- Tool calls and results preserved in conversation history
- Pattern: SessionManager passed to ContextBuilder constructor

**From Story 2.5 (Sessions Directory Setup)**:
- Session file format defined: JSON with snake_case fields
- Session ID format: "{channel}_{chat_id}"
- File permissions: 0600
- FIFO behavior: 50 messages max, oldest removed when full

### Git Intelligence Summary

**Recent Commits**:
- `cf7894d` - fix(agent): resolve all code review issues in ContextBuilder
- `6388eee` - Add agent loop implementation
- `9ec457e` - Update message tool to use ChatHub and add context support
- `4a85174` - Add message tool to agent tools
- `cfcc27a` - Add delivery failure callback and improve outbound message handling
- `364ca03` - Add chat hub implementation
- `c0d90be` - Add session management module and tests

**Code Patterns from Recent Work**:
- AgentLoop uses tokio::select! for message processing
- SessionManager exists with basic structure (from commit c0d90be)
- ContextBuilder integrated with AgentLoop
- Error handling uses anyhow/thiserror pattern

### Latest Tech Information

**Dependencies to Use**:
- `tokio::sync::RwLock` - For concurrent session access (already in project)
- `tokio::time::interval` - For persistence loop timing
- `tokio::fs` - For async file operations
- `chrono::serde` - For timestamp serialization (already used)
- `serde_json` - For JSON serialization (already used)

**No external research needed** - All technologies already established in project.

## Dev Agent Record

### Agent Model Used

- Model: Claude (OpenAI)
- Date: 2026-02-15
- Story: 5.3 - Session Management in Agent Loop

### Debug Log References

**Code Review Findings (2026-02-15)**:
- Fixed HIGH: Tool result role mismatch in ContextBuilder - added support for "tool_result" role mapping to LlmRole::Tool
- Fixed HIGH: Added integration test verifying tool_calls survive full persistence→recovery→ContextBuilder pipeline
- Fixed MEDIUM: Removed mysterious EOF file from project root
- Fixed MEDIUM: Enhanced error handling for persistence failures with detailed logging and retry notes
- Fixed MEDIUM: Added performance notes about session cloning in save_session method
- Fixed MEDIUM: Added integration test verifying session updates are visible to ContextBuilder across iterations
- Fixed MEDIUM: Documented intentional "tool_result" vs "tool" role naming distinction
- All 2 HIGH and 5 MEDIUM issues resolved ✅

### Completion Notes List

1. ✅ **Task 1: SessionManager Integration**
   - AgentLoop already received SessionManager in constructor
   - `get_or_create_session` properly handles session lifecycle
   - User messages added to session at start of `process_message`
   - Assistant messages added during `run_agent_loop`
   - Updated `save_session` to use new `update_session` and `persist_session` methods

2. ✅ **Task 2: Session Lifecycle Hooks**
   - Extended `Message` type with helper methods: `tool_result()`, `is_user()`, `is_assistant()`, `is_tool_result()`
   - Modified `run_agent_loop` to store tool_calls in assistant messages
   - Tool results stored as separate messages with "tool_result" role
   - FIFO limit of 50 messages enforced by VecDeque in Session type

3. ✅ **Task 3: Persistence Integration**
   - Added `update_session()` and `persist_session()` methods to SessionManager
   - Auto-persistence already implemented via `start_auto_persistence()` (30s interval)
   - `save_session` in AgentLoop now calls both update and persist for immediate saves
   - Error handling prevents persistence failures from crashing agent loop

4. ✅ **Task 4: Session Recovery**
   - Session loading on startup already implemented in `SessionManager::initialize()`
   - Loads all sessions from `~/.miniclaw/workspace/sessions/`
   - Handles corrupted files by renaming to `.corrupted` and creating new session
   - Logs session count on initialization

5. ✅ **Task 5: ContextBuilder Integration**
   - ContextBuilder already receives Session via `build_context(&session, &message)`
   - Session history (max 50 messages) passed to ContextBuilder
   - Tool calls preserved in messages for LLM context
   - Session updates visible on next iteration via cloned session updates

6. ✅ **Task 6: Testing**
   - Added 12 unit tests in `agent_loop.rs` for session lifecycle and tool interactions
   - Added 7 integration tests in `session_integration_tests.rs` for persistence, recovery, concurrent access, and context integration
   - All tests passing (100% success rate)
   - Added comprehensive tool_calls pipeline test (persistence → recovery → ContextBuilder)
   - Added session visibility test verifying updates across agent loop iterations

### File List

**Modified Files:**
1. `src/session/types.rs` - Added Message helper methods; documented tool_result role naming
2. `src/session/manager.rs` - Added update_session() and persist_session() methods
3. `src/agent/agent_loop.rs` - Modified run_agent_loop to store tool_calls and tool_results; enhanced save_session with error handling and performance notes
4. `src/agent/context.rs` - Fixed tool_result role mapping to support LlmRole::Tool

**Created Files:**
1. `tests/session_integration_tests.rs` - Integration tests for persistence, recovery, concurrent access, tool_calls pipeline, and session visibility

---

**Story Created**: 2026-02-15
**Epic**: 5 - Agent Core - Loop & Context
**Dependencies**: Story 5.1 (Agent Loop Implementation), Story 5.2 (Context Builder), Story 2.5 (Sessions Directory Setup)
**Blocks**: Story 6.x (Tool implementations that need session context), Story 9.x (Session Management advanced features)

**Completion Note**: Ultimate context engine analysis completed - comprehensive developer guide created
