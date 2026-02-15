# Story 4.1: LLM Provider Trait and Architecture

Status: done

## Story

As a developer,
I want a provider-agnostic interface for LLMs,
so that miniclaw can work with multiple LLM services.

## Acceptance Criteria

1. **Trait Definition**: Given the provider module is implemented, when defining the LLMProvider trait, then it requires `chat()` method for completions, and it requires `default_model()` method, and it is Send + Sync for thread safety

2. **Chat Method Signature**: Given the chat method signature, when implementing a provider, then it accepts: messages (Vec<Message>), tools (Vec<ToolDefinition>), model (&str), and returns: Result<LLMResponse>

3. **Message Type**: Given the Message type, when creating conversation messages, then it has role enum: System, User, Assistant, and has content field (String), and optionally has tool_calls field

4. **LLMResponse Type**: Given the LLMResponse type, when parsing provider responses, then it contains content (String, assistant message), and optionally contains tool_calls (Vec<ToolCall>), and contains usage statistics (prompt_tokens, completion_tokens)

5. **ToolCall Type**: Given the ToolCall type, when the LLM requests tool execution, then it contains id (unique identifier), and contains name (tool name), and contains arguments (JSON object as string)

6. **Provider Factory**: Given multiple providers may exist, when selecting a provider, then a factory pattern allows runtime provider selection, and configuration determines which provider to instantiate

7. **Error Handling**: Given provider errors occur, when handling failures, then errors are categorized (network, auth, rate limit, invalid request), and retryable errors are distinguished from fatal errors

## Tasks / Subtasks

- [x] **Task 1**: Extract and refine LLMProvider trait from agent_loop.rs (AC: #1)
  - [x] 1.1 Move LLMProvider trait definition to src/providers/mod.rs
  - [x] 1.2 Ensure Send + Sync bounds are correctly applied
  - [x] 1.3 Add comprehensive documentation to trait methods
  - [x] 1.4 Update imports in agent_loop.rs to use new location

- [x] **Task 2**: Define core provider types (AC: #2, #3, #4, #5)
  - [x] 2.1 Define Message struct with role, content, tool_calls fields
  - [x] 2.2 Define Role enum (System, User, Assistant, Tool)
  - [x] 2.3 Define LLMResponse struct with content, tool_calls, usage stats
  - [x] 2.4 Define ToolCall struct with id, name, arguments
  - [x] 2.5 Define ToolDefinition type for tool schemas
  - [x] 2.6 Implement serialization/deserialization for all types

- [x] **Task 3**: Create ProviderError enum (AC: #7)
  - [x] 3.1 Define error variants: Network, Auth, RateLimit, InvalidRequest, Timeout
  - [x] 3.2 Implement std::error::Error trait
  - [x] 3.3 Add retryable() method to identify retryable errors
  - [x] 3.4 Add Display implementation with clear messages

- [x] **Task 4**: Implement ProviderFactory (AC: #6)
  - [x] 4.1 Define ProviderConfig enum with variants for each provider type
  - [x] 4.2 Create ProviderFactory with create() method
  - [x] 4.3 Support OpenRouter configuration
  - [x] 4.4 Support Ollama configuration
  - [x] 4.5 Add validation for required configuration fields

- [x] **Task 5**: Add provider module structure (AC: #1)
  - [x] 5.1 Create src/providers/ directory
  - [x] 5.2 Create mod.rs with trait and type exports
  - [x] 5.3 Create error.rs for ProviderError
  - [x] 5.4 Create factory.rs for ProviderFactory
  - [x] 5.5 Update main lib.rs to include providers module

- [x] **Task 6**: Create mock provider for testing (AC: #1, #2)
  - [x] 6.1 Implement MockLlmProvider for unit tests
  - [x] 6.2 Support configurable responses
  - [x] 6.3 Support simulating errors
  - [x] 6.4 Add to test utilities

- [x] **Task 7**: Testing and validation
  - [x] 7.1 Unit tests for all provider types (serialization, deserialization)
  - [x] 7.2 Unit tests for ProviderError (retryable detection)
  - [x] 7.3 Unit tests for ProviderFactory (configuration parsing)
  - [x] 7.4 Integration tests with MockLlmProvider
  - [x] 7.5 Test error handling paths

## Dev Notes

### Architecture Compliance

**LLMProvider Trait Pattern** (MUST follow exactly) [Source: architecture.md#API Boundaries]:
- The trait is already defined in Story 5.1 (agent_loop.rs) and MUST be preserved
- All implementations must be Send + Sync for thread-safe concurrent usage
- The chat() method is async and takes ownership of messages and tools
- Model parameter allows runtime model selection per request

**Type Definitions from Story 5.1** [Source: src/agent/agent_loop.rs:42-85]:
```rust
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
    pub tool_calls: Option<Vec<LlmToolCall>>,
}

pub enum LlmRole { System, User, Assistant, Tool }

pub struct LlmToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Option<Vec<LlmToolCall>>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
}
```

**Error Handling Strategy** [Source: architecture.md#Process Patterns]:
- Use thiserror for error definitions (library code)
- Categorize errors for appropriate handling (retry vs fail)
- Network errors and rate limits are retryable
- Auth errors and invalid requests are not retryable

**Factory Pattern** [Source: architecture.md#Implementation Patterns]:
- Use factory pattern for runtime provider selection
- Configuration-driven provider instantiation
- Support multiple provider instances simultaneously

### Source Tree Components to Touch

**Files to Create**:
1. `src/providers/mod.rs` - Main provider module with trait and types
2. `src/providers/error.rs` - ProviderError definition
3. `src/providers/factory.rs` - ProviderFactory implementation
4. `src/providers/mock.rs` - Mock provider for testing

**Files to Modify**:
1. `src/agent/agent_loop.rs` - Remove trait definition, import from providers
2. `src/agent/mod.rs` - Update exports to use providers module
3. `src/lib.rs` - Add providers module
4. `src/config/mod.rs` - Add provider configuration types

**Dependencies**:
- `async-trait` - Already in use for async traits
- `serde` - For JSON serialization
- `thiserror` - For error definitions

### Testing Standards Summary

**Test Coverage Required**:
- Trait bounds verification (Send + Sync)
- Type serialization/deserialization
- Error categorization (retryable vs fatal)
- Factory configuration parsing
- Mock provider behavior

**Test Patterns**:
- Unit tests co-located with implementation
- Mock provider for dependent component tests
- Integration tests for factory configuration

### Project Structure Notes

**Expected Structure**:
```
src/providers/
├── mod.rs          # Trait and type definitions
├── error.rs        # ProviderError
├── factory.rs      # ProviderFactory
└── mock.rs         # MockLlmProvider for tests
```

**Integration Points**:
- AgentLoop uses LlmProvider trait
- Configuration loads provider settings
- Future stories implement concrete providers (OpenRouter, Ollama)

### Detected Conflicts or Variances

**Variance Alert**: The LlmProvider trait is currently defined in `src/agent/agent_loop.rs` from Story 5.1. 

**Resolution**: Extract the trait definition to `src/providers/mod.rs` and update imports. This is a refactoring task, not new development. The trait definition is already complete and tested.

## References

### Technical Details with Source Paths

**LLMProvider Trait**:
- [Source: src/agent/agent_loop.rs:87-100] - Current trait definition (to be moved)
- [Source: architecture.md#API Boundaries] - Trait pattern requirements

**Supporting Types**:
- [Source: src/agent/agent_loop.rs:42-85] - LlmMessage, LlmRole, LlmToolCall, LlmResponse
- [Source: architecture.md#Data Structures] - Type requirements

**Error Handling**:
- [Source: architecture.md#Process Patterns] - Error categorization strategy
- [Source: architecture.md#Error Handling] - Retryable vs fatal errors

**Factory Pattern**:
- [Source: architecture.md#Implementation Patterns] - Factory pattern usage

### Previous Story Intelligence

**From Story 5.1 (Agent Loop)** [Source: 5-1-agent-loop-implementation.md]:
- LLMProvider trait already defined and tested
- Types (LlmMessage, LlmRole, etc.) already defined
- Integration with AgentLoop established
- Mock provider created for unit tests

**Patterns Established**:
- Async trait with `#[async_trait::async_trait]`
- Send + Sync requirements for thread safety
- Error propagation using AgentError
- Structured logging with tracing

### Latest Tech Information

**Dependencies Already in Project**:
- `async-trait = "0.1"` - For async traits
- `serde = { version = "1.0", features = ["derive"] }` - For serialization
- `thiserror = "1.0"` - For error definitions
- `tokio = { version = "1", features = ["full"] }` - For async runtime

**No New Dependencies Required** - All technologies already established in project.

## Dev Agent Record

### Agent Model Used

- Model: k2p5 (claude-opus-4-5-20251101)
- Date: 2026-02-15
- Story: 4.1 - LLM Provider Trait and Architecture

### Debug Log References

No debug logs required - implementation completed successfully

### Code Review Fixes Applied (2026-02-15)

**Review by:** k2p5 (claude-sonnet-4.5-20250214)
**Issues Found:** 1 High, 5 Medium, 3 Low
**Fixes Applied:** All High and Medium issues resolved

**High Priority Fixes:**
- H1: Fixed memory leak in MockLlmProvider::provider_name() - Changed from Box::leak() to storing &'static str directly

**Medium Priority Fixes:**
- M1: Added OllamaConfig and OpenRouterConfig to public exports
- M2: Added comprehensive documentation explaining Vec<serde_json::Value> vs ToolDefinition design choice
- M3: Types already public in mod.rs (LlmMessage, LlmRole, etc.) - no additional export needed
- M4: Fixed clippy warning in test_mock_provider_reset - added `let _ =` to handle Result
- M5: Config types now exported publicly from factory module

**Low Priority Issues (Noted, Not Fixed):**
- L1: provider_name() signature with &'static str is now correct after H1 fix
- L2: ProviderError::Unknown variant documented as intentional catch-all
- L3: Integration tests deferred to future stories using real providers

### Completion Notes List

1. **Task 1: Extract and refine LLMProvider trait** (AC: #1)
   - Extracted trait from agent_loop.rs to src/providers/mod.rs
   - Added Send + Sync bounds and comprehensive documentation
   - Updated imports in agent_loop.rs and context.rs
   - Added provider_name() method to trait

2. **Task 2: Define core provider types** (AC: #2, #3, #4, #5)
   - Defined LlmMessage with helper methods (is_system, is_user, etc.)
   - Defined LlmRole enum with serialization support
   - Defined LlmToolCall with parse_arguments() helper
   - Defined LlmResponse with token tracking helpers
   - Defined ToolDefinition with OpenAI format conversion
   - All types implement Serialize/Deserialize for JSON support

3. **Task 3: Create ProviderError enum** (AC: #7)
   - Implemented 8 error variants: Network, Auth, RateLimit, InvalidRequest, Timeout, Provider, Serialization, Config, Unknown
   - Added is_retryable() method for retry logic
   - Added is_auth_error() and is_rate_limit() helpers
   - Added retry_after() for retry timing
   - Implemented From<serde_json::Error>

4. **Task 4: Implement ProviderFactory** (AC: #6)
   - Created OpenRouterConfig with builder pattern and validation
   - Created OllamaConfig with builder pattern and validation
   - Created ProviderConfig enum with serde tag-based serialization
   - Implemented ProviderFactory::create() with placeholder for future providers
   - Added ProviderFactory::create_from_json() for JSON configuration

5. **Task 5: Add provider module structure** (AC: #1)
   - Created src/providers/ directory
   - Created mod.rs with all exports and 19 unit tests
   - Created error.rs with ProviderError and 14 unit tests
   - Created factory.rs with configs and 17 unit tests
   - Created mock.rs with MockLlmProvider and 15 unit tests
   - Updated src/lib.rs to include providers module
   - Updated src/agent/mod.rs to re-export from providers

6. **Task 6: Create mock provider** (AC: #1, #2)
   - Implemented MockLlmProvider with configurable responses
   - Added call tracking (call_count, last_messages)
   - Added error simulation support
   - Created MockProviderBuilder for fluent configuration
   - All mock features tested

7. **Task 7: Testing and validation**
   - 65+ unit tests across all modules
   - Tests for serialization, deserialization, error handling
   - Tests for factory configuration and validation
   - All tests passing (100% success rate)

### File List

**New Files:**
1. `src/providers/mod.rs` - LlmProvider trait and core types (399 lines, 19 tests)
2. `src/providers/error.rs` - ProviderError enum (290 lines, 14 tests)
3. `src/providers/factory.rs` - Provider configs and factory (461 lines, 17 tests)
4. `src/providers/mock.rs` - MockLlmProvider for testing (358 lines, 15 tests)

**Modified Files:**
1. `src/lib.rs` - Added `pub mod providers;`
2. `src/agent/mod.rs` - Re-exports from providers module
3. `src/agent/agent_loop.rs` - Removed local trait/types, import from providers
4. `src/agent/context.rs` - Import LlmMessage, LlmRole, LlmToolCall from providers
5. `src/providers/mod.rs` - Added config exports, improved documentation (Code Review)
6. `src/providers/mock.rs` - Fixed memory leak in provider_name(), fixed clippy warning (Code Review)

---

**Story Created**: 2026-02-15
**Epic**: 4 - LLM Provider Integration
**Dependencies**: Story 5.1 (Agent Loop - provides trait definition to extract)
**Blocks**: Story 4.2 (OpenAI-Compatible Provider), Story 4.3 (Ollama Local Provider), Story 4.4 (Agent One-Shot Command)

**Completion Note**: This story primarily involves refactoring the existing LLMProvider trait from agent_loop.rs into a dedicated providers module, then building the factory and error handling infrastructure around it.
