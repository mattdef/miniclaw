# Story 3.3: Outbound Message Delivery

Status: done

## Story

As an agent,
I want to send replies back to users,
So that I can communicate responses and results.

## Acceptance Criteria

1. **Outbound Routing** (AC: 1)
   - [x] **Given** the agent generates a response
   - [x] **When** it sends the message to Chat Hub
   - [x] **Then** the Hub creates an OutboundMessage
   - [x] **And** routes it to the correct channel

2. **Protocol Adherence** (AC: 2)
   - [x] **Given** an outbound message
   - [x] **When** it is delivered to the channel adapter
   - [x] **Then** the channel-specific protocol is used
   - [x] **And** for Telegram: uses teloxide send_message (handled in Epic 10)

3. **Delivery Failure Handling** (AC: 3)
   - [x] **Given** a message delivery fails
   - [x] **When** the channel returns an error
   - [x] **Then** the error is logged with level ERROR
   - [x] **And** the message may be retried based on channel policy
   - [x] **And** the agent is notified of delivery failure

4. **Outbound Buffer Management** (AC: 4)
   - [x] **Given** the outbound buffer is full
   - [x] **When** a new message needs to be sent
   - [x] **Then** the system waits briefly for space
   - [x] **And** if still full, drops oldest message
   - [x] **And** logs a warning

5. **Threaded Replies** (AC: 5)
   - [x] **Given** a reply references a specific message
   - [x] **When** it is sent via Telegram
   - [x] **Then** it uses reply_to_message_id for threading

## Tasks / Subtasks

- [x] Enhance `ChatHub::send_outbound` with robust retry logic
  - [x] Implement exponential backoff placeholder/logic (logged for now)
- [x] Implement delivery status notification
  - [x] Add feedback mechanism via `Result` and logging
- [x] Refine `ChatHub::run` for outbound routing
  - [x] Ensure `route_outbound` is called for all messages in the outbound channel
- [x] Add unit tests for outbound delivery
  - [x] Test routing to registered channels
  - [x] Test handling of missing channels
  - [x] Test buffer overflow for outbound
- [x] Add integration tests in `tests/chat_tests.rs`

## Dev Notes

### Retry Policy

For the initial implementation, we will log errors. Complex retry policies will be added as needed when channel adapters are implemented.

### Buffer Management

Similar to inbound, we use a manual FIFO drop strategy to satisfy the non-blocking requirement while maintaining a bounded buffer.

## Change Log

**2026-02-15:** Created Story 3.3 - Outbound Message Delivery
**2026-02-15:** Code review fixes - Added exponential backoff retry, delivery failure notifications, wait briefly logic, and comprehensive tests

## Dev Agent Record

### File List

**Modified Files:**
- `src/chat/hub.rs` - Core ChatHub implementation with outbound routing, retry logic, and buffer management
- `src/chat/types.rs` - OutboundMessage type definitions
- `tests/chat_tests.rs` - Integration tests for outbound message delivery

### Implementation Summary

1. **Outbound Routing** (`ChatHub::route_outbound`): Routes messages to registered channel adapters with exponential backoff retry (3 attempts with 100ms, 200ms, 400ms delays)
2. **Delivery Failure Handling**: Errors logged at ERROR level with agent notification via callback mechanism (`on_delivery_failure`)
3. **Buffer Management** (`send_outbound`): Waits 100ms for space before dropping oldest message when buffer full
4. **Threaded Replies**: `reply_to` helper method sets `reply_to` field for Telegram threading support

### Tests Added

- `test_buffer_overflow_outbound`: Verifies buffer overflow handling for outbound messages
- `test_delivery_failure_notification`: Verifies agent callback is invoked on delivery failure
- `test_route_outbound`: Basic routing test
- `test_route_outbound_unregistered_channel`: Tests error handling for missing channels
