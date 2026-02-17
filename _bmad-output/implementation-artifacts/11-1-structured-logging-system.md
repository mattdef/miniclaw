# Story 11.1: Structured Logging System

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want comprehensive logging,
so that I can debug and monitor system behavior.

## Acceptance Criteria

1. **Given** the logging system
   **When** initialized at startup
   **Then** it uses tracing crate for structured logging
   **And** configures subscriber with formatting

2. **Given** log levels
   **When** logging events
   **Then** supports: ERROR, WARN, INFO, DEBUG, TRACE
   **And** default level is INFO

3. **Given** ERROR level
   **When** used
   **Then** logs failures requiring intervention
   **And** examples: crash risks, data loss, auth failures

4. **Given** WARN level
   **When** used
   **Then** logs unexpected but handled situations
   **And** examples: rate limits, retries, deprecated usage

5. **Given** INFO level
   **When** used
   **Then** logs important lifecycle events
   **And** examples: startup, config loaded, connection established

6. **Given** DEBUG level
   **When** used (verbose mode)
   **Then** logs detailed operation info
   **And** examples: tool execution, context assembly, API calls

7. **Given** TRACE level
   **When** used (very verbose)
   **Then** logs extremely detailed data
   **And** examples: serialization, raw API responses

8. **Given** log format
   **When** writing to stderr
   **Then** includes timestamp (ISO 8601)
   **And** includes level
   **And** includes target module
   **And** includes message

9. **Given** structured logging
   **When** including fields
   **Then** uses tracing key-value pairs
   **And** example: `info!(user_id = %id, "Message received")`

10. **Given** secret protection
    **When** logging configuration or API data
    **Then** API keys are NEVER logged
    **And** tokens are NEVER logged
    **And** passwords are NEVER logged
    **And** only existence is logged: "API key configured: true"

## Tasks / Subtasks

- [x] Task 1: Initialize tracing subscriber in main.rs (AC: 1)
  - [x] Add tracing_subscriber initialization with EnvFilter
  - [x] Configure default level to INFO
  - [x] Support RUST_LOG environment variable
  - [x] Configure stderr as output target
- [x] Task 2: Implement --verbose flag handling (AC: 2, 6)
  - [x] Parse --verbose flag from CLI arguments
  - [x] Set DEBUG level when --verbose is present
  - [x] Document verbose mode in help text
- [x] Task 3: Configure log format (AC: 8)
  - [x] Include ISO 8601 timestamp
  - [x] Include log level
  - [x] Include target module path
  - [x] Include message
  - [x] Use human-readable format (not JSON for CLI)
- [x] Task 4: Add structured logging examples (AC: 9)
  - [x] Instrument ChatHub with message received logs
  - [x] Instrument AgentLoop with iteration logs
  - [x] Instrument tool execution with parameters
  - [x] Use key-value pairs for contextual data
- [x] Task 5: Implement secret protection (AC: 10)
  - [x] Review Config struct for sensitive fields
  - [x] Add helper methods for safe config access
  - [x] Create SafeConfigSummary for logging without secrets
  - [x] Never use debug! or trace! with raw config objects containing secrets
- [x] Task 6: Add logging to existing components (AC: 3, 4, 5, 6, 7)
  - [x] Add INFO logs for startup sequence
  - [x] Add INFO logs for config loading with safe summary
  - [x] Add WARN logs for retry attempts
  - [x] Add ERROR logs for failures
  - [x] Add DEBUG logs for tool execution (verbose only)

## Dev Notes

### Architecture Compliance

**Required Patterns:**
- Use `tracing` crate for all logging (already in Cargo.toml)
- Use `tracing_subscriber` with `env-filter` feature for level configuration
- Output logs to stderr, NOT stdout (stdout reserved for command results)
- Use structured fields: `info!(field = %value, "message")` instead of format strings
- Never use `println!` or `eprintln!` for logging purposes

**Log Level Guidelines:**
```rust
// ERROR: System failures, crashes, data loss risks
tracing::error!(error = %e, "Failed to persist session");

// WARN: Recoverable issues, unexpected conditions
tracing::warn!(attempt = retry_count, "Retrying failed request");

// INFO: Normal lifecycle events (visible to users)
tracing::info!("Configuration loaded from {}", config_path);

// DEBUG: Detailed operation info (verbose mode only)
tracing::debug!(tool = %tool_name, "Executing tool");

// TRACE: Very detailed data (very verbose, avoid in production)
tracing::trace!(response = %raw_json, "Raw API response");
```

### Secret Protection Implementation

```rust
// BAD - Never do this
tracing::debug!("Config: {:?}", config);  // May leak API keys!

// GOOD - Log only what's needed
tracing::info!(
    openai_configured = !config.openai_api_key.is_empty(),
    telegram_configured = !config.telegram_token.is_empty(),
    "API configuration status"
);
```

### Integration Points

**Components to instrument:**
- `main.rs`: Startup logging, initialization sequence
- `Config::load()`: Configuration loading status
- `ChatHub::run()`: Message processing lifecycle
- `AgentLoop::process_message()`: Agent iteration tracking
- `Tool::execute()`: Tool execution flow
- `SessionManager`: Session persistence events
- `Gateway`: Daemon mode lifecycle

### File Structure Notes

**No new files needed** - logging integration goes into existing modules.

**Modified files:**
- `src/main.rs`: Add subscriber initialization
- `src/config/mod.rs`: Add config loading logs
- `src/chat/hub.rs`: Add message processing logs
- `src/agent/mod.rs`: Add agent loop logs
- `src/tools/mod.rs`: Add tool execution logs
- `src/session/manager.rs`: Add session lifecycle logs
- `src/gateway.rs`: Add gateway lifecycle logs

### References

**Source Documents:**
- [Source: epics.md#Story 11.1: Structured Logging System] - Full acceptance criteria
- [Source: architecture.md#Logging with tracing] - Logging patterns and examples
- [Source: architecture.md#Enforcement Guidelines] - Rules for structured logging
- [Source: prd.md#FR45-47] - Functional requirements for logging

**External Documentation:**
- Tracing crate: https://docs.rs/tracing/latest/tracing/
- Tracing subscriber: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/
- Best practices: Use EnvFilter for RUST_LOG support, format to stderr

## Dev Agent Record

### Agent Model Used

Opencode CLI - kimi-for-coding/k2p5

### Debug Log References

### Completion Notes List

1. **Task 1: Initialize tracing subscriber in main.rs**
   - Added explicit `with_writer(std::io::stderr)` to ensure logs go to stderr
   - Already had EnvFilter with RUST_LOG support
   - Already had --verbose flag integration
   - Default level INFO, DEBUG when verbose enabled

2. **Task 2: Implement --verbose flag handling**
   --verbose flag already existed in CLI (global flag)
   - Tests verify -v and --verbose work correctly
   - Help text already documented verbose mode

3. **Task 3: Configure log format**
   - Log format includes ISO 8601 timestamp, level, module path, message
   - Verified output goes to stderr (not stdout)
   - Human-readable format for CLI tool

4. **Task 4: Add structured logging examples**
   - ChatHub already has tracing for message processing and routing
   - AgentLoop already has comprehensive structured logging for iterations, tool calls, and responses
   - Tool registry enhanced with execution logging
   - All use key-value pairs: `info!(field = %value, "message")`

5. **Task 5: Implement secret protection**
   - Added `SafeConfigSummary` struct for safe logging
   - Added helper methods: `is_api_key_configured()`, `is_telegram_configured()`, `is_model_configured()`
   - Config loading now uses `get_safe_summary()` for logging
   - Never logs actual secret values, only boolean flags

6. **Task 6: Add logging to existing components**
   - Gateway: Comprehensive lifecycle logging (startup, shutdown, components)
   - Session Manager: Auto-persistence and cleanup logging with DEBUG/WARN levels
   - Agent Loop: Iteration tracking, LLM calls, tool execution, TRACE for raw responses
   - Chat Hub: Message routing, retry attempts, failures, structured message received logs
   - Config Loader: Multi-layer loading with safe summary, ERROR level for permission issues
   - Tool Registry: Tool execution start/success/failure

### Code Review Fixes Applied

After adversarial code review, the following issues were identified and fixed:

1. **AC #6-7 TRACE Level Logging** (HIGH)
   - Added TRACE logging for raw LLM API responses in `src/agent/agent_loop.rs:306-312`
   - Added TRACE logging for context serialization in `src/agent/context.rs:478-486`
   - Now properly demonstrates TRACE level usage as specified in AC #7

2. **Task 4 Structured Logging in ChatHub** (HIGH)
   - Added structured INFO log with channel, chat_id, message_length fields in `src/chat/hub.rs:99-104`
   - Matches AC #9 pattern: `info!(field = %value, "Message received")`

3. **Session Manager Logging Enhancement** (MEDIUM)
   - Added DEBUG logs for session lookup operations
   - Added WARN logs for failed persistence operations with failure counts
   - Improved observability for verbose mode debugging

4. **Config Loader Security Logging** (MEDIUM)
   - Changed permissions check from WARN to ERROR level (AC #3 compliance)
   - Security-related issues now logged as ERROR per acceptance criteria

5. **File List Documentation** (HIGH)
   - Updated to include all actually modified files
   - Added sprint-status.yaml to match git reality
   - Added session/manager.rs to document logging enhancements

### File List

Modified files:
- `src/main.rs` - Explicit stderr output for logging
- `src/config/schema.rs` - SafeConfigSummary and helper methods for secret protection
- `src/config/loader.rs` - Enhanced logging for config loading with safe summary; permissions check now ERROR level
- `src/agent/tools/mod.rs` - Tool execution logging
- `src/agent/agent_loop.rs` - Added TRACE logging for raw LLM API responses
- `src/agent/context.rs` - Added TRACE logging for context serialization
- `src/chat/hub.rs` - Added structured INFO logging for message received events
- `src/session/manager.rs` - Added DEBUG/WARN logs for session operations and auto-persistence
- `_bmad-output/implementation-artifacts/sprint-status.yaml` - Updated story status to review

---

**Story Context Quality Notes:**

This story provides the logging infrastructure foundation for the entire system. All subsequent stories (11.2-11.4) depend on this being implemented first. The tracing setup must be complete before implementing output stream management (11.2) because the logging system determines stderr/stdout behavior.

**Technical Decisions Already Made:**
1. Use `tracing` + `tracing-subscriber` crates (already in Cargo.toml)
2. Use EnvFilter for environment-based level control
3. Output to stderr (not stdout)
4. Human-readable format for CLI tool (not JSON)
5. Default level: INFO
6. Support --verbose flag for DEBUG level

**Previous Story Intelligence:**
- This is the first story in Epic 11, no previous stories to learn from
- However, stories 1-1 through 10-3 are complete - review their error handling patterns
- Pattern established: Use anyhow for CLI, thiserror for libraries
- Pattern established: All I/O operations are async

**Common Mistakes to Avoid:**
1. Don't use println!/eprintln! for logging - use tracing macros
2. Don't forget to mask secrets in logs - audit all Config usage
3. Don't log at wrong level - ERROR for failures, WARN for issues, INFO for lifecycle
4. Don't forget --verbose flag integration with clap
5. Don't output logs to stdout - must go to stderr
6. Don't forget RUST_LOG environment variable support

**Testing Requirements:**
- Test with `RUST_LOG=debug cargo run` - verify DEBUG logs appear
- Test with `--verbose` flag - verify DEBUG logs appear
- Test with `RUST_LOG=warn` - verify only WARN and ERROR appear
- Test config loading - verify no secrets in logs
- Test piping: `cargo run -- version 2>/dev/null` - verify no logs in stdout
