# Story 3.2: Inbound Message Processing

Status: done

## Story

As an agent,
I want to receive and process incoming messages,
So that I can respond to user requests.

## Acceptance Criteria

1. **Inbound Message Format Conversion** (AC: 1)
   - [x] **Given** a message arrives from Telegram
   - [x] **When** it is received by the Chat Hub
   - [x] **Then** it is converted to InboundMessage format
   - [x] **And** timestamp is recorded (ISO 8601 UTC)

2. **Message Extraction** (AC: 2)
   - [x] **Given** an inbound message
   - [x] **When** the agent loop processes it
   - [x] **Then** the message content is extracted
   - [x] **And** the chat_id is identified for routing replies
   - [x] **And** the channel is identified for protocol-specific handling

3. **Whitespace Filtering** (AC: 3)
   - [x] **Given** a message contains only whitespace
   - [x] **When** it is processed
   - [x] **Then** it is ignored (not sent to agent)
   - [x] **And** no reply is generated

4. **Message Length Management** (AC: 4)
   - [x] **Given** a message is too long (>4000 characters for Telegram)
   - [x] **When** it is processed
   - [x] **Then** it is truncated or split appropriately
   - [x] **And** a warning is logged

5. **Concurrency Handling** (AC: 5)
   - [x] **Given** concurrent messages from multiple users
   - [x] **When** they arrive simultaneously
   - [x] **Then** each is processed independently
   - [x] **And** no messages are lost
   - [x] **And** replies are routed to correct users

## Tasks / Subtasks

- [x] Implement message validation and sanitization
  - [x] Add `sanitize()` method to `InboundMessage`
  - [x] Implement whitespace-only check
  - [x] Implement content trimming
- [x] Enhance `ChatHub::send_inbound` with processing logic
  - [x] Apply validation before queuing
  - [x] Log warnings for ignored/truncated messages
- [x] Implement message length limits
  - [x] Add constant for max message length (e.g., 4000)
  - [x] Implement truncation logic in `InboundMessage`
- [x] Update `ChatHub::run` to handle message processing
  - [x] Ensure independent processing of messages
- [x] Add unit tests for message processing
  - [x] Test whitespace-only messages are ignored
  - [x] Test message truncation
  - [x] Test concurrent message handling
- [x] Add integration tests in `tests/chat_tests.rs`

## Dev Notes

### Truncation Strategy

For the MVP, we will truncate messages that exceed the limit and log a warning. In future stories, we might implement splitting.

### Concurrency

Tokio's `mpsc` and `select!` already handle concurrent arrivals. The "independent processing" will be more relevant once we have an Agent state to manage.

## Dev Agent Record

### File List

**Modified Files:**
- `src/lib.rs` - Added chat module export

**Created Files:**
- `src/chat/mod.rs` - Chat module exports
- `src/chat/types.rs` - InboundMessage and OutboundMessage types with sanitization
- `src/chat/hub.rs` - ChatHub with message routing and processing
- `tests/chat_tests.rs` - Integration tests for ChatHub functionality
- `tests/chat_types_tests.rs` - Unit tests for message types

### Change Log

**2026-02-15:** Created Story 3.2 - Inbound Message Processing
**2026-02-15:** Code review completed, fixed test coverage gaps and documentation
