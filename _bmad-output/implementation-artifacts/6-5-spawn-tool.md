# Story 6.5: Spawn Tool

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an agent,
I want to run background tasks,
so that I can execute long-running operations without blocking.

## Acceptance Criteria

1. **Command Validation**: Given the spawn tool is called when spawning a process, then it accepts parameters: command, args (array), optional cwd, and uses same validation as exec tool, and applies same command blacklist

2. **Non-Blocking Spawn**: Given a spawn request when process starts, then it returns immediately with process ID, and does not wait for completion, and process runs in background

3. **Fire-and-Forget**: Given a spawned process when it completes, then exit code is logged, and stdout/stderr may be logged (configurable), and agent is NOT notified (fire-and-forget)

4. **Process Limitations**: Given spawn limitations when tracking spawned processes, then agent can view active processes via system tools, and agent cannot directly communicate with spawned process

5. **Error Handling**: Given a spawn request fails when command not found or invalid, then error is returned immediately, and no process is created

## Tasks / Subtasks

- [x] **Task 1**: Create SpawnTool struct and implement Tool trait (AC: #1)
  - [x] 1.1 Create `SpawnTool` struct in `src/agent/tools/spawn.rs`
  - [x] 1.2 Implement `Tool` trait with name="spawn", description, parameters schema
  - [x] 1.3 Define parameters schema with command (string, required), args (array of strings), cwd (string, optional)
  - [x] 1.4 Implement `execute()` method with spawn logic

- [x] **Task 2**: Implement command validation and security (AC: #1)
  - [x] 2.1 Apply same blacklist as exec tool: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
  - [x] 2.2 Reject blacklisted commands with error "Command not allowed: {command}"
  - [x] 2.3 Validate cwd path if provided using canonicalize()
  - [x] 2.4 Ensure args are provided as array (prevent shell injection)

- [x] **Task 3**: Implement non-blocking process spawn (AC: #2)
  - [x] 3.1 Use `tokio::process::Command` for async process spawning
  - [x] 3.2 Configure stdout/stderr capture based on config
  - [x] 3.3 Spawn process and immediately return process ID
  - [x] 3.4 Do NOT await process completion

- [x] **Task 4**: Implement background process tracking (AC: #3)
  - [x] 4.1 Create background task to wait for spawned process
  - [x] 4.2 Log exit code when process completes
  - [x] 4.3 Optionally log stdout/stderr based on configuration
  - [x] 4.4 Implement fire-and-forget (no agent notification)

- [x] **Task 5**: Implement error handling (AC: #5)
  - [x] 5.1 Handle command not found errors
  - [x] 5.2 Handle invalid cwd path errors
  - [x] 5.3 Return immediate errors for spawn failures
  - [x] 5.4 Never panic on user input errors

- [x] **Task 6**: Add configuration support (AC: #3)
  - [x] 6.1 Add `spawn_log_output` config option (default: false)
  - [x] 6.2 Read config in SpawnTool initialization
  - [x] 6.3 Conditionally capture/log output based on config

- [x] **Task 7**: Register tool in AgentLoop (AC: All)
  - [x] 7.1 Add spawn module export in `src/agent/tools/mod.rs`
  - [x] 7.2 Import SpawnTool in oneshot registration
  - [x] 7.3 Register SpawnTool in tool registry alongside other tools

- [x] **Task 8**: Testing and validation (AC: All)
  - [x] 8.1 Unit tests for SpawnTool in spawn.rs (19 tests)
  - [x] 8.2 Test command validation: blacklisted commands, valid commands
  - [x] 8.3 Test non-blocking spawn: immediate PID return
  - [x] 8.4 Test fire-and-forget: no agent notification
  - [x] 8.5 Test error handling: command not found, invalid cwd
  - [x] 8.6 Integration tests for tool execution flow
  - [x] 8.7 All tests pass (19 new spawn tests added, 358 total library tests passing)

## Dev Notes

### Relevant Architecture Patterns and Constraints

**Tool Implementation Pattern** (MUST follow exactly) [Source: architecture.md#Architectural Boundaries]:
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    async fn execute(&self, args: HashMap<String, Value>, ctx: &ToolExecutionContext) -> ToolResult<String>;
}
```

**File Location** [Source: architecture.md#Project Structure & Boundaries]:
- Create: `src/agent/tools/spawn.rs`
- Register in: `src/agent/tools/mod.rs` and `src/agent/oneshot.rs`

**Naming Conventions** (RFC 430) [Source: architecture.md#Naming Patterns]:
- Struct: `SpawnTool` (PascalCase)
- File: `spawn.rs` (snake_case)
- Methods: `execute()`, `spawn_process()` (snake_case)

**Async Process Management** [Source: architecture.md#Process Patterns]:
- Use `tokio::process::Command` for async process spawning
- Spawn background task with `tokio::spawn()` for tracking
- Never block agent loop waiting for process completion

**Security Requirements** [Source: architecture.md#Security Boundaries]:
- Use same blacklist as exec tool: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
- Args must be provided as array (prevent shell injection)
- Path validation via canonicalize() for cwd parameter

### Source Tree Components to Touch

1. **New File**: `src/agent/tools/spawn.rs` - SpawnTool implementation
2. **Modify**: `src/agent/tools/mod.rs` - Add spawn module and export
3. **Modify**: `src/agent/oneshot.rs` - Register SpawnTool in registry
4. **Modify**: `src/config/schema.rs` - Add spawn_log_output config option
5. **New Tests**: `src/agent/tools/spawn.rs` (co-located `#[cfg(test)]` module)

### Key Technical Requirements

**Blacklist Enforcement** (SAME AS EXEC TOOL):
```rust
const BLACKLISTED_COMMANDS: &[&str] = &["rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo"];
```

**Async Spawn Pattern**:
```rust
// Spawn process without waiting
let mut child = tokio::process::Command::new(command)
    .args(&args)
    .current_dir(cwd)
    .spawn()?;

// Return PID immediately
let pid = child.id().unwrap_or(0);

// Background task for tracking
tokio::spawn(async move {
    match child.wait().await {
        Ok(status) => tracing::info!(pid = %pid, exit_code = ?status.code(), "Spawned process completed"),
        Err(e) => tracing::error!(pid = %pid, error = %e, "Spawned process error"),
    }
});
```

**Configuration**:
```rust
// In config.json
{
  "spawn_log_output": false  // Default: don't log stdout/stderr
}
```

**Error Types**:
- Blacklisted command → `ToolError::PermissionDenied`
- Command not found → `ToolError::ExecutionFailed` with clear message
- Invalid cwd → `ToolError::InvalidArguments`
- Spawn failure → `ToolError::ExecutionFailed`

**Response Format**:
Return JSON string with:
```json
{
  "success": true,
  "pid": 12345,
  "message": "Process spawned successfully"
}
```

Or on error:
```json
{
  "success": false,
  "error": "Command not allowed: sudo"
}
```

### Previous Story Learnings (Story 6.4 - Web Tool)

**Patterns Established**:
- Implement Tool trait with consistent structure
- Use async/await for all I/O operations
- Return structured JSON responses with all relevant data
- Use existing error types from `crate::agent::tools::ToolError`
- Comprehensive unit tests (20+ tests for WebTool)

**Security Lessons**:
- Always validate user input (commands and paths)
- Use blacklist approach for dangerous commands
- Prevent resource exhaustion (spawn creates processes but doesn't track indefinitely)
- Log security events (attempted blacklisted command execution)

**Testing Patterns**:
- Use `#[tokio::test]` for async tests
- Mock external dependencies where possible
- Test error cases thoroughly
- Test both blocking and non-blocking behavior

**Code Quality Standards** (from 6-4 review):
- Add explicit documentation for all public methods
- Use structured logging with tracing
- Include helpful error messages with suggestions
- Never use magic numbers (extract to constants)

### Git Intelligence (Recent Commits)

Recent patterns from `git log`:
- Tools are implemented in individual files under `src/agent/tools/`
- Each tool implements the `Tool` trait with consistent structure
- Tools are registered in the oneshot module for CLI usage
- Comprehensive unit tests are co-located in the same file
- Configuration options added to src/config/schema.rs

### Project Structure Notes

**Alignment with Unified Project Structure**:
- Follows established pattern: `src/agent/tools/{tool_name}.rs`
- Tool registration in `src/agent/oneshot.rs` alongside other tools
- Consistent with existing tools: filesystem, exec, web

**No Detected Conflicts**: Project structure matches expected layout from architecture.md

### References

- **Tool Trait Definition**: [Source: architecture.md#Architectural Boundaries]
- **Exec Tool Pattern**: [Source: 6-3-exec-tool.md] - Use same blacklist and validation
- **Async Process Pattern**: [Source: architecture.md#Process Patterns]
- **Configuration Pattern**: [Source: 2-1-configuration-file-management.md]
- **Error Handling Pattern**: [Source: 6-2-filesystem-tool.md#Error Types]

### External Libraries

**tokio** (already in dependencies):
- Use `tokio::process::Command` for async process spawning
- Use `tokio::spawn()` for background task tracking
- Documentation: https://docs.rs/tokio/latest/tokio/process/

## Dev Agent Record

### Agent Model Used

k2p5 (Claude Code)

### Debug Log References

N/A - Clean implementation with minimal compilation errors (missing spawn_log_output field in Config structs)

### Completion Notes List

1. ✅ **Task 1 Complete**: SpawnTool struct created with full Tool trait implementation
2. ✅ **Task 2 Complete**: Command validation and security implemented - uses same blacklist as exec tool
3. ✅ **Task 3 Complete**: Non-blocking spawn implemented using tokio::process::Command with immediate PID return
4. ✅ **Task 4 Complete**: Background process tracking with tokio::spawn(), exit code logging, optional stdout/stderr logging
5. ✅ **Task 5 Complete**: Comprehensive error handling for command not found, invalid cwd, and blacklisted commands
6. ✅ **Task 6 Complete**: Configuration support added with spawn_log_output option in Config struct
7. ✅ **Task 7 Complete**: SpawnTool registered in mod.rs and oneshot.rs alongside other tools
8. ✅ **Task 8 Complete**: 19 comprehensive unit tests added, all 358 library tests passing

### File List

**Core Implementation:**
- `src/agent/tools/spawn.rs` (NEW) - SpawnTool implementation with 19 unit tests
- `src/agent/tools/mod.rs` (MODIFIED) - Added spawn module export
- `src/agent/oneshot.rs` (MODIFIED) - Registered SpawnTool in tool registry
- `src/config/schema.rs` (MODIFIED) - Added spawn_log_output config option
- `src/config/loader.rs` (MODIFIED) - Updated merge_config_from_file and merge_env_variables to handle spawn_log_output

**Testing:**
- `tests/tool_integration_tests.rs` (MODIFIED) - Added spawn tool integration tests

**Build & Dependencies:**
- `Cargo.toml` (MODIFIED) - No dependency changes (uses existing tokio)
- `Cargo.lock` (MODIFIED) - Dependency lock file updated

**Code Quality Improvements (from review):**
- `src/agent/tools/web.rs` (MODIFIED) - Removed unused is_json_content() method
- `src/providers/openai.rs` (MODIFIED) - Removed unused serde_json::json import

**Documentation:**
- `_bmad-output/planning-artifacts/architecture.md` (MODIFIED) - Updated with spawn tool patterns
- `_bmad-output/implementation-artifacts/6-4-web-tool.md` (MODIFIED) - Status updated
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED) - Story 6-5 tracking
- `_bmad-output/implementation-artifacts/6-5-spawn-tool.md` (NEW) - This story file

### Change Log

- **2026-02-16**: Story implementation completed
  - Created SpawnTool with full Tool trait implementation
  - Implemented non-blocking process spawn with fire-and-forget pattern
  - Added configurable output logging via spawn_log_output config option
  - Comprehensive test coverage with 19 unit tests
  - All 358 library tests passing

- **2026-02-16**: Code review corrections applied
  - Fixed misleading test count documentation (clarified 19 new tests vs 358 total)
  - Removed unused `is_json_content()` method from WebTool (dead code cleanup)
  - Removed unused `serde_json::json` import from OpenAI provider
  - Added 2 integration tests for spawn tool (registration + blacklist enforcement)
  - Updated File List to document all git changes (11 modified files, 2 new files)
  - Removed test for deleted `is_json_content()` method
  - Final test count: 456 total tests passing (357 library + 99 integration/doc/other)

