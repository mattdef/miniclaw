# Story 4.3: Ollama Local Provider

Status: done

## Story

As a privacy-conscious user,
I want to use local LLMs via Ollama,
so that my data never leaves my machine.

## Acceptance Criteria

1. **Ollama Connection**: Given Ollama is running locally on port 11434, when miniclaw is configured with Ollama provider, then it makes HTTP requests to `http://localhost:11434/api/chat`, and uses the Ollama chat API format

2. **Chat API Format**: Given Ollama provider is selected, when calling the chat endpoint, then it sends model name (e.g., "llama3.2", "mistral"), and sends messages in Ollama format, and optionally sends tools if model supports it

3. **Streaming Response Handling**: Given Ollama returns a streaming response, when processing the chunks, then it accumulates the full response, and handles the stream properly, and returns complete message when done

4. **Connection Error Handling**: Given Ollama is not running, when attempting to connect, then it detects connection refused error, and suggests "Is Ollama running? Start it with: ollama serve", and provides clear troubleshooting steps

5. **Model Availability**: Given the requested model is not available, when Ollama returns 404, then it suggests running `ollama pull [model_name]`, and lists available models via `ollama list`

6. **Token Usage Estimation**: Given a local LLM is used, when tracking token usage, then it estimates tokens if provider doesn't return them, and logs that usage is approximate

## Tasks / Subtasks

- [x] **Task 1**: Create Ollama provider implementation file (AC: #1, #2)
  - [x] 1.1 Create `src/providers/ollama.rs` file
  - [x] 1.2 Implement `OllamaProvider` struct with base_url (default: http://localhost:11434)
  - [x] 1.3 Implement `LLMProvider` trait for OllamaProvider
  - [x] 1.4 Add reqwest client for local HTTP communication

- [x] **Task 2**: Implement Ollama chat API request format (AC: #1, #2)
  - [x] 2.1 Build Ollama-compatible request body from messages
  - [x] 2.2 Handle Ollama-specific message format (different from OpenAI)
  - [x] 2.3 Add optional tools support (for models that support function calling)
  - [x] 2.4 Set correct API endpoint URL (/api/chat)

- [x] **Task 3**: Implement streaming response handling (AC: #3)
  - [x] 3.1 Configure reqwest client for streaming response handling
  - [x] 3.2 Accumulate response chunks from stream
  - [x] 3.3 Parse JSON lines from stream
  - [x] 3.4 Extract complete message when stream ends

- [x] **Task 4**: Implement error handling for Ollama (AC: #4, #5)
  - [x] 4.1 Handle connection refused errors
  - [x] 4.2 Handle 404 model not found errors
  - [x] 4.3 Provide helpful error messages with suggestions
  - [x] 4.4 Handle streaming errors gracefully

- [x] **Task 5**: Implement token estimation (AC: #6)
  - [x] 5.1 Add token estimation for Ollama responses (4 chars ≈ 1 token approximation)
  - [x] 5.2 Log when using estimated token counts
  - [x] 5.3 Track both prompt and completion tokens

- [x] **Task 6**: Add provider configuration support
  - [x] 6.1 Add OllamaConfig to factory.rs
  - [x] 6.2 Ensure ProviderFactory can instantiate OllamaProvider
  - [x] 6.3 Support custom base_url configuration
  - [x] 6.4 Support default model configuration

- [x] **Task 7**: Testing and validation
  - [x] 7.1 Unit tests for request building
  - [x] 7.2 Unit tests for response parsing
  - [x] 7.3 Unit tests for streaming accumulation
  - [x] 7.4 Unit tests for error handling
  - [x] 7.5 Mock tests with simulated Ollama responses
  - [x] 7.6 Integration test with real Ollama (optional, marked with `#[ignore]`)

## Dev Notes

### Architecture Compliance

**LLMProvider Trait Implementation** (MUST follow exactly) [Source: src/providers/mod.rs]:
- Must implement `LLMProvider` trait with `chat()`, `default_model()`, and `provider_name()` methods
- Implementation must be Send + Sync for thread-safe concurrent usage
- The chat() method is async and takes Vec<LlmMessage>, Vec<ToolDefinition>, and model string
- Must return Result<LlmResponse, ProviderError>

**Provider Types from Story 4.1** [Source: src/providers/mod.rs]:
```rust
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
    pub tool_calls: Option<Vec<LlmToolCall>>,
    pub tool_call_id: Option<String>,
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
- Categorize errors for appropriate handling
- Connection errors are retryable (Ollama might restart)
- Model not found errors are not retryable
- Map all errors to ProviderError enum from src/providers/error.rs

**Provider Configuration** [Source: src/providers/factory.rs]:
- Add OllamaConfig struct with:
  - base_url: String (default: http://localhost:11434)
  - default_model: String (e.g., "llama3.2")
- ProviderFactory::create() handles instantiation
- No API key required for local Ollama

### Source Tree Components to Touch

**Files to Create**:
1. `src/providers/ollama.rs` - Ollama provider implementation

**Files to Modify**:
1. `src/providers/mod.rs` - Add `pub mod ollama;` and export OllamaProvider
2. `src/providers/factory.rs` - Add OllamaConfig and OllamaProvider instantiation
3. `src/providers/error.rs` - Add Ollama-specific error variants if needed

**Dependencies** (already in project):
- `reqwest = { version = "0.12", features = ["json", "stream"] }` - HTTP client with streaming support
- `serde = { version = "1.0", features = ["derive"] }` - JSON serialization
- `serde_json = "1.0"` - JSON parsing
- `tokio = { version = "1", features = ["full"] }` - Async runtime
- `tokio-stream` - Stream utilities for response handling

### Testing Standards Summary

**Test Coverage Required**:
- Request JSON serialization matches Ollama API format
- Response parsing handles streaming chunks correctly
- Error handling for connection refused
- Error handling for model not found (404)
- Token estimation accuracy
- Trait implementation verification

**Test Patterns**:
- Unit tests co-located in `#[cfg(test)]` module at bottom of ollama.rs
- Use MockLlmProvider pattern from src/providers/mock.rs for testing
- Mock HTTP responses using reqwest mock or custom test doubles
- Integration tests marked with `#[ignore]` for real Ollama testing

### Project Structure Notes

**Expected Structure**:
```
src/providers/
├── mod.rs          # Trait and type definitions (update)
├── error.rs        # ProviderError (update if needed)
├── factory.rs      # ProviderFactory (update)
├── mock.rs         # MockLlmProvider
├── openai.rs       # OpenRouterProvider implementation
└── ollama.rs       # NEW: OllamaProvider implementation
```

**Integration Points**:
- Implements LLMProvider trait defined in Story 4.1
- Uses ProviderError for error handling (same as OpenRouter)
- Registered in ProviderFactory for runtime instantiation
- Used by AgentLoop for LLM calls (same as OpenRouter)
- Can be configured alongside OpenRouter for user choice

### References

### Technical Details with Source Paths

**Ollama API**:
- Base URL: http://localhost:11434 (default)
- Chat endpoint: /api/chat
- Documentation: https://github.com/ollama/ollama/blob/main/docs/api.md
- Authentication: None required for local instance

**Ollama Request Format**:
```json
{
  "model": "llama3.2",
  "messages": [
    {"role": "system", "content": "..."},
    {"role": "user", "content": "..."}
  ],
  "stream": true,
  "tools": [...]  // Optional, for models that support tools
}
```

**Ollama Streaming Response Format**:
Each line is a JSON object:
```json
{"model":"llama3.2","created_at":"2026-02-15T10:30:00Z","message":{"role":"assistant","content":"Hello"},"done":false}
```
Final message:
```json
{"model":"llama3.2","created_at":"2026-02-15T10:30:00Z","message":{"role":"assistant","content":""},"done":true,"total_duration":123456789,"load_duration":12345678,"prompt_eval_count":50,"eval_count":100}
```

**Key Differences from OpenAI API**:
1. Ollama uses streaming by default (stream field)
2. Response format is NDJSON (newline-delimited JSON)
3. Token counts in final chunk only
4. No Authorization header needed
5. Local HTTP (not HTTPS) by default

**ProviderError Usage** [Source: src/providers/error.rs]:
- `ProviderError::Network` - For HTTP/network errors including connection refused
- `ProviderError::NotFound` - For 404 model not found errors
- `ProviderError::Timeout` - For request timeouts
- `ProviderError::InvalidRequest` - For 4xx client errors
- `ProviderError::Provider` - For 5xx server errors

### Previous Story Intelligence

**From Story 4.2 (OpenAI-Compatible Provider)** [Source: 4-2-openai-compatible-provider.md]:
- OpenRouter provider successfully implemented and reviewed
- Pattern for LLMProvider trait implementation established
- Error handling with retry logic working well
- ProviderFactory pattern proven

**Patterns Established**:
- Async trait with `#[async_trait::async_trait]`
- Send + Sync requirements for thread safety
- Error propagation using ProviderError
- Structured logging with tracing
- Builder pattern for configuration
- reqwest client with configurable timeouts
- Exponential backoff for retryable errors

**Files Created in Story 4.2**:
- `src/providers/openai.rs` - OpenRouter provider (720 lines, 18 tests)
- Modified factory.rs, mod.rs, error.rs for OpenRouter support

**Lessons Learned from Code Review**:
1. Add `try_new()` method for fallible client creation (don't panic)
2. Include retry attempt count in error messages
3. Handle tool_call_id for OpenAI-compatible Tool role messages
4. Never log full request objects (may contain secrets)
5. Use Network error type instead of hardcoded Timeout for generic network errors

**Apply to Ollama Provider**:
- Use same error handling patterns
- Implement try_new() for OllamaProvider
- Follow same logging practices (no sensitive data)
- Reuse ProviderError variants
- Similar test coverage expectations

### Latest Tech Information

**Dependencies Already in Project**:
- `reqwest = { version = "0.12", features = ["json", "stream"] }` - HTTP client with JSON and streaming
- `serde = { version = "1.0", features = ["derive"] }` - Serialization
- `serde_json = "1.0"` - JSON handling
- `tokio = { version = "1", features = ["full"] }` - Async runtime
- `tokio-stream = "0.1"` - Stream utilities
- `tracing = "0.1"` - Structured logging

**Ollama API Details**:
- Default port: 11434
- Chat endpoint: POST /api/chat
- Streaming is default behavior
- Supports many models: llama3.2, mistral, codellama, etc.
- Tool support varies by model (llama3.2 supports tools)

**Token Estimation Strategy**:
- Ollama provides token counts in final stream chunk
- For estimation when not available: 1 token ≈ 4 characters
- Log warning when using estimated counts
- Track separately: prompt_tokens and completion_tokens

**Streaming Implementation**:
- Use reqwest's `bytes_stream()` method
- Parse each line as JSON using serde_json
- Accumulate content from message fields
- Stop when `done: true` received

**Error Handling Strategy**:
- Connection refused: "Is Ollama running? Start it with: ollama serve"
- Model not found (404): "Model 'X' not found. Run: ollama pull X"
- Network timeout: Standard 30s timeout with retry
- Streaming errors: Log and return accumulated content or error

## Dev Agent Record

### Agent Model Used

k2p5 (claude-opus-4-5-20251101)

### Debug Log References

No debug logs required - implementation completed successfully

### Code Review Fixes (2026-02-15)

Following adversarial code review, the following issues were identified and fixed:

**HIGH Priority Fixes:**
1. **Added tool_call_id support** (AC #2 compliance) - `OllamaMessage` now includes `tool_call_id` field for Tool role messages, matching OpenRouter implementation
2. **Fixed token estimation** (AC #6 compliance) - Changed from using Debug format (`"{:?}"`) to proper message content iteration with new `estimate_message_tokens()` method
3. **Factory panic prevention** - Changed `ProviderFactory::create()` to use `OllamaProvider::try_new()` instead of panicking `new()`
4. **Stream completion validation** (AC #3 compliance) - Added warning when stream ends without `done=true` flag

**MEDIUM Priority Fixes:**
1. **Performance optimization** - Changed `Bytes::from(line.to_vec())` to `Bytes::copy_from_slice(line)` to avoid unnecessary allocation
2. **Options serialization error handling** - Changed from silent `unwrap_or_default()` to explicit warning on serialization failure
3. **Test coverage** - Added `test_empty_stream_response()` for edge case testing

**LOW Priority Fixes:**
1. **Test clarity** - Updated comment in `test_build_request_with_empty_model` to clarify behavior

**Result:** All 249 tests passing (18 Ollama tests including 1 ignored integration test)

### Completion Notes List

1. **Task 1: Create Ollama provider implementation file** (AC: #1, #2)
   - Created `src/providers/ollama.rs` with complete OllamaProvider implementation
   - Implemented `OllamaProvider` struct with base_url, default_model, timeout_seconds
   - Implemented `LLMProvider` trait with chat(), default_model(), and provider_name() methods
   - Added reqwest client for local HTTP communication with configurable timeout

2. **Task 2: Implement Ollama chat API request format** (AC: #1, #2)
   - Created `OllamaRequest`, `OllamaMessage`, and `OllamaToolCall` structs for API format
   - Built request builder that converts LlmMessage to Ollama format
   - Added optional tools support for models that support function calling
   - Set correct API endpoint URL (/api/chat) with base_url configuration

3. **Task 3: Implement streaming response handling** (AC: #3)
   - Configured reqwest client for streaming with `bytes_stream()` method
   - Implemented `OllamaResponseChunk` struct for parsing streaming JSON
   - Created `AccumulatedResponse` struct to accumulate content and token counts
   - Implemented accumulation logic that handles multiple JSON objects in one chunk
   - Properly handles the final chunk with token counts and done flag

4. **Task 4: Implement error handling for Ollama** (AC: #4, #5)
   - Handle connection refused errors with helpful message: "Is Ollama running? Start it with: ollama serve"
   - Handle 404 model not found with suggestion: "Run `ollama pull [model_name]`"
   - Map HTTP errors to appropriate ProviderError variants
   - Handle streaming errors gracefully without crashing

5. **Task 5: Implement token estimation** (AC: #6)
   - Extract token counts from final chunk when available (prompt_eval_count, eval_count)
   - Implement token estimation fallback: 1 token ≈ 4 characters
   - Log warning when using estimated token counts
   - Track both prompt and completion tokens separately

6. **Task 6: Add provider configuration support**
   - OllamaConfig already existed in factory.rs
   - Updated ProviderFactory::create() to instantiate OllamaProvider
   - Support custom base_url configuration (default: http://localhost:11434)
   - Support default model configuration (default: llama3.2)
   - Support additional options HashMap for Ollama-specific parameters

7. **Task 7: Testing and validation**
   - 17 unit tests covering all major functionality
   - Tests for request building with/without tools
   - Tests for response parsing (success, errors, streaming)
   - Tests for token estimation
   - Tests for error handling (connection, HTTP, model not found)
   - Tests for trait implementation verification
   - Integration test marked with `#[ignore]` for real Ollama testing
   - All tests passing (248 total tests in codebase)

### File List

**New Files:**
1. `src/providers/ollama.rs` - Ollama provider implementation (800+ lines, 18 tests including 1 ignored integration test)

**Modified Files:**
1. `src/providers/mod.rs` - Added `pub mod ollama;` and export `OllamaProvider`
2. `src/providers/factory.rs` - Updated ProviderFactory to create OllamaProvider using `try_new()`, updated test
3. `Cargo.toml` - Added `bytes = "1.0"` dependency and added `"stream"` feature to reqwest
4. `Cargo.lock` - Updated dependencies lock file

