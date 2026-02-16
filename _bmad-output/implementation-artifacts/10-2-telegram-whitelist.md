# Story 10.2: Telegram Whitelist

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want to restrict who can interact with my agent,
So that only authorized users can access it.

## Acceptance Criteria

1. **Whitelist Configuration** (AC: 1)
   - **Given** whitelist configuration
   - **When** configured in config.json
   - **Then** it has field: `allow_from: [123456789, 987654321]`
   - **And** contains array of allowed Telegram user IDs

2. **User ID Extraction** (AC: 2)
   - **Given** incoming message
   - **When** checking whitelist
   - **Then** it extracts user_id from message.from.id
   - **And** checks if in allow_from list

3. **Whitelisted User Processing** (AC: 3)
   - **Given** user is whitelisted
   - **When** message passes check
   - **Then** it is processed normally
   - **And** routed to agent

4. **Non-Whitelisted User Rejection** (AC: 4)
   - **Given** user is NOT whitelisted
   - **When** message fails check
   - **Then** message is silently dropped
   - **And** not sent to agent
   - **And** DEBUG log shows "Message from non-whitelisted user {id}"

5. **Empty Whitelist Security** (AC: 5)
   - **Given** empty whitelist
   - **When** allow_from is [] or missing
   - **Then** all messages are rejected (secure by default)
   - **And** logs warn "Whitelist empty, no users allowed"

6. **Onboarding Integration** (AC: 6)
   - **Given** whitelist management
   - **When** onboarding runs
   - **Then** it asks for user ID
   - **And** adds it to allow_from automatically
   - **And** explains how to add more users later

## Tasks / Subtasks

- [x] Add allow_from field to Config struct (AC: 1)
  - [x] Add `allow_from: Vec<i64>` field to Config struct
  - [x] Add serde deserialization with default empty vector
  - [x] Update config validation to handle optional field
- [x] Implement whitelist checking in Telegram adapter (AC: 2-5)
  - [x] Extract user_id from Telegram message.from.id
  - [x] Create whitelist validation function (WhitelistChecker in security.rs)
  - [x] Implement secure-by-default rejection logic
  - [x] Add DEBUG logging for rejected messages
  - [x] Add WARN logging for empty whitelist
- [x] Update onboarding to configure whitelist (AC: 6)
  - [x] Add user ID prompt during onboarding
  - [x] Explain how to find Telegram user ID (@userinfobot)
  - [x] Add user ID to allow_from in config
  - [x] Display confirmation of whitelist configuration
- [x] Write unit tests
  - [x] Test whitelist validation with valid user IDs
  - [x] Test rejection of non-whitelisted users
  - [x] Test secure-by-default with empty whitelist
  - [x] Test config serialization/deserialization with allow_from
  - [x] Test config validation for invalid user IDs
- [x] Write integration tests
  - [x] Test end-to-end whitelist filtering
  - [x] Test TelegramChannel creation with whitelist
  - [x] Test environment variable parsing for allow_from

## Dev Notes

### Architecture Compliance

**Security Module Extension (per architecture.md):**

The whitelist functionality should extend the existing security module at `src/utils/security.rs`:

```rust
// src/utils/security.rs
pub struct WhitelistChecker {
    allowed_users: Vec<i64>,
}

impl WhitelistChecker {
    pub fn new(allowed_users: Vec<i64>) -> Self {
        Self { allowed_users }
    }
    
    pub fn is_allowed(&self, user_id: i64) -> bool {
        if self.allowed_users.is_empty() {
            // Secure by default: empty whitelist = no one allowed
            tracing::warn!("Whitelist empty, no users allowed");
            return false;
        }
        self.allowed_users.contains(&user_id)
    }
    
    pub fn add_user(&mut self, user_id: i64) {
        if !self.allowed_users.contains(&user_id) {
            self.allowed_users.push(user_id);
        }
    }
}
```

**Config Struct Update:**

```rust
// src/config/mod.rs
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    // ... existing fields ...
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub allow_from: Vec<i64>, // Telegram user IDs allowed to interact
    // ... other fields ...
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        // ... existing validation ...
        
        // Validate allow_from entries are positive integers
        for user_id in &self.allow_from {
            if *user_id <= 0 {
                return Err(anyhow!("Invalid user ID in allow_from: {}", user_id));
            }
        }
        
        Ok(())
    }
}
```

**Telegram Adapter Integration:**

```rust
// src/channels/telegram.rs
use crate::utils::security::WhitelistChecker;

pub struct TelegramChannel {
    bot: Bot,
    whitelist: WhitelistChecker,
}

impl TelegramChannel {
    pub fn new(token: String, allowed_users: Vec<i64>) -> Result<Self> {
        // ... existing token validation ...
        
        let whitelist = WhitelistChecker::new(allowed_users);
        
        Ok(Self {
            bot,
            whitelist,
        })
    }
    
    async fn handle_message(&self, msg: Message) -> Option<InboundMessage> {
        // Extract user_id from message
        let user_id = msg.from.as_ref()?.id.0 as i64;
        
        // Check whitelist
        if !self.whitelist.is_allowed(user_id) {
            tracing::debug!("Message from non-whitelisted user {}", user_id);
            return None;
        }
        
        // User is whitelisted, create InboundMessage
        let chat_id = msg.chat.id.to_string();
        let content = msg.text().unwrap_or("").to_string();
        
        Some(InboundMessage {
            channel: "telegram".to_string(),
            chat_id,
            content,
            metadata: {
                let mut map = HashMap::new();
                map.insert("user_id".to_string(), user_id.to_string());
                if let Some(username) = &msg.from.as_ref()?.username {
                    map.insert("username".to_string(), username.clone());
                }
                map
            },
        })
    }
}
```

**Update Handler with Whitelist:**

```rust
// In TelegramChannel::start() method
let handler = Update::filter_message().endpoint(
    move |msg: Message, bot: Bot| {
        let inbound_tx = inbound_tx.clone();
        let whitelist = whitelist.clone(); // Arc<WhitelistChecker> if needed
        
        async move {
            // Check whitelist before processing
            if let Some(user) = &msg.from {
                let user_id = user.id.0 as i64;
                
                if !whitelist.is_allowed(user_id) {
                    tracing::debug!("Message from non-whitelisted user {}", user_id);
                    return;
                }
            } else {
                tracing::debug!("Message without user info, dropping");
                return;
            }
            
            // Process whitelisted message
            let chat_id = msg.chat.id.to_string();
            let content = msg.text().unwrap_or("").to_string();
            
            let inbound = InboundMessage {
                channel: "telegram".to_string(),
                chat_id,
                content,
                metadata: HashMap::new(),
            };
            
            if let Err(e) = inbound_tx.send(inbound).await {
                tracing::error!("Failed to send inbound message: {}", e);
            }
        }
    }
);
```

**Onboarding Flow Update:**

```rust
// In src/commands/onboard.rs
async fn configure_telegramWhitelist(config: &mut Config) -> Result<()> {
    println!("\n📱 Configuration Telegram - Sécurité");
    println!("Pour sécuriser votre agent, vous devez spécifier qui peut interagir avec lui.");
    
    // Explain how to get user ID
    println!("\nPour trouver votre ID Telegram :");
    println!("  1. Ouvrez Telegram et cherchez @userinfobot");
    println!("  2. Cliquez sur 'Démarrer'");
    println!("  3. Le bot affichera votre ID (ex: 123456789)");
    println!("  4. Copiez cet ID ci-dessous");
    
    // Prompt for user ID
    let user_id: i64 = Text::new("Votre ID Telegram:")
        .with_validator(|input: &str| {
            match input.parse::<i64>() {
                Ok(id) if id > 0 => Ok(Validation::Valid),
                _ => Ok(Validation::Invalid(
                    "L'ID doit être un nombre positif".into()
                )),
            }
        })
        .prompt()?;
    
    // Add to whitelist
    config.allow_from.push(user_id);
    
    println!("\n✅ ID {} ajouté à la whitelist", user_id);
    println!("Seuls les utilisateurs dans cette liste pourront interagir avec l'agent.");
    println!("Vous pouvez ajouter d'autres utilisateurs en modifiant ~/.miniclaw/config.json");
    
    Ok(())
}
```

### Source Tree Components

**Files to Create:**

1. No new files needed - extends existing modules

**Files to Modify:**

1. `src/utils/security.rs` - Add WhitelistChecker
   - Add WhitelistChecker struct
   - Implement is_allowed() method
   - Implement secure-by-default logic

2. `src/config/mod.rs` - Add allow_to field
   - Add `allow_from: Vec<i64>` field
   - Add serde default attribute
   - Update validation logic

3. `src/channels/telegram.rs` - Integrate whitelist
   - Add whitelist field to TelegramChannel
   - Update constructor to accept allowed_users
   - Add whitelist check in message handler
   - Extract user_id from message.from.id

4. `src/commands/onboard.rs` - Add whitelist configuration
   - Add user ID prompt during onboarding
   - Add explanation of how to find user ID
   - Automatically add user ID to allow_from

5. `src/gateway.rs` - Pass whitelist to TelegramChannel
   - Update TelegramChannel::new() call to include config.allow_from

**Data Flow with Whitelist:**

```
┌─────────────────┐
│  Telegram API   │
│  Update::Message│
└────────┬────────┘
         │
         ▼
┌──────────────────────────────────────────┐
│  TelegramChannel Dispatcher              │
│  ┌──────────────────────────────────┐    │
│  │  1. Extract user_id              │    │
│  │     from message.from.id         │    │
│  └────────────┬─────────────────────┘    │
│               │                          │
│               ▼                          │
│  ┌──────────────────────────────────┐    │
│  │  2. Whitelist Check              │    │
│  │  ┌──────────────────────────┐    │    │
│  │  │ allow_from empty?        │    │    │
│  │  │ YES → Reject (warn log)  │    │    │
│  │  │ NO  → Check user_id      │    │    │
│  │  │       in list?           │    │    │
│  │  │       YES → Allow        │    │    │
│  │  │       NO  → Reject       │    │    │
│  │  │            (debug log)   │    │    │
│  │  └──────────────────────────┘    │    │
│  └────────────┬─────────────────────┘    │
│               │                          │
│    ┌──────────┴──────────┐               │
│    ▼                   ▼               │
│ ┌──────────────┐   ┌──────────────┐     │
│ │   ALLOWED    │   │  REJECTED    │     │
│ │ Create       │   │ Silently     │     │
│ │ InboundMsg   │   │ drop message │     │
│ └──────┬───────┘   └──────────────┘     │
└────────┼─────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│   ChatHub       │
│  inbound_tx     │
└─────────────────┘
```

### Testing Standards

**Unit Tests (in `src/utils/security.rs`):**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_whitelist_allows_configured_users() {
        let checker = WhitelistChecker::new(vec![123456789, 987654321]);
        assert!(checker.is_allowed(123456789));
        assert!(checker.is_allowed(987654321));
    }
    
    #[test]
    fn test_whitelist_rejects_unknown_users() {
        let checker = WhitelistChecker::new(vec![123456789]);
        assert!(!checker.is_allowed(999999999));
    }
    
    #[test]
    fn test_empty_whitelist_rejects_all() {
        let checker = WhitelistChecker::new(vec![]);
        assert!(!checker.is_allowed(123456789));
        assert!(!checker.is_allowed(987654321));
    }
    
    #[test]
    fn test_add_user_to_whitelist() {
        let mut checker = WhitelistChecker::new(vec![]);
        checker.add_user(123456789);
        assert!(checker.is_allowed(123456789));
    }
    
    #[test]
    fn test_add_duplicate_user_idempotent() {
        let mut checker = WhitelistChecker::new(vec![123456789]);
        checker.add_user(123456789); // Already exists
        assert_eq!(checker.allowed_users.len(), 1);
    }
}
```

**Unit Tests (in `src/config/mod.rs`):**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_deserialize_with_allow_from() {
        let json = r#"{
            "api_key": "test",
            "telegram_token": "123:abc",
            "allow_from": [123456789, 987654321]
        }"#;
        
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.allow_from, vec![123456789, 987654321]);
    }
    
    #[test]
    fn test_config_deserialize_without_allow_from() {
        let json = r#"{
            "api_key": "test",
            "telegram_token": "123:abc"
        }"#;
        
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.allow_from.is_empty()); // Default to empty
    }
    
    #[test]
    fn test_config_validate_rejects_invalid_user_id() {
        let config = Config {
            // ... other fields ...
            allow_from: vec![-1, 0], // Invalid: negative or zero
        };
        
        assert!(config.validate().is_err());
    }
}
```

**Integration Tests (in `tests/telegram_whitelist_tests.rs`):**

```rust
#[tokio::test]
async fn test_whitelisted_user_message_processed() {
    let allowed_users = vec![123456789];
    let channel = TelegramChannel::new(
        "123456789:test_token".to_string(),
        allowed_users
    ).unwrap();
    
    // Mock Telegram message from whitelisted user
    let message = create_mock_message(123456789);
    
    // Should return Some(InboundMessage)
    let result = channel.handle_message(message).await;
    assert!(result.is_some());
}

#[tokio::test]
async fn test_non_whitelisted_user_message_rejected() {
    let allowed_users = vec![123456789];
    let channel = TelegramChannel::new(
        "123456789:test_token".to_string(),
        allowed_users
    ).unwrap();
    
    // Mock Telegram message from non-whitelisted user
    let message = create_mock_message(999999999);
    
    // Should return None (message dropped)
    let result = channel.handle_message(message).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_empty_whitelist_rejects_all() {
    let channel = TelegramChannel::new(
        "123456789:test_token".to_string(),
        vec![] // Empty whitelist
    ).unwrap();
    
    let message = create_mock_message(123456789);
    
    // Should return None (secure by default)
    let result = channel.handle_message(message).await;
    assert!(result.is_none());
}
```

### Project Structure Notes

**Alignment with Unified Project Structure:**

- Security functionality follows existing pattern: `src/utils/security.rs`
- Config extension follows existing pattern: add field with serde support
- Channel modification follows Story 10.1 pattern
- Onboarding update follows Story 2.2 pattern

**No Detected Conflicts:** Project structure supports all required modifications

**Integration with Current System:**

- **Story 10.1 (Telegram Bot Adapter)**: MUST be completed - provides TelegramChannel
- **Story 2.2 (Interactive Onboarding)**: Extends existing onboarding flow
- **Config Module**: Already supports extensible configuration
- **Security Module**: Already exists for exec blacklist, extend for whitelist

**Dependencies:**

- **Story 10.1**: REQUIRED - TelegramChannel structure and message handling
- **Story 2.2**: OPTIONAL - If onboarding exists, extend it; otherwise document manual config

### Previous Story Intelligence

**From Story 10.1 (Telegram Bot Adapter):**

- TelegramChannel uses teloxide's Dispatcher with Update::filter_message()
- Message handler receives `Message` type from teloxide
- User ID available via `msg.from.as_ref()?.id.0` (returns u64, cast to i64)
- Channel registration pattern with ChatHub already established
- Token validation and error handling patterns established

**Key Implementation Details from 10.1:**

```rust
// From Story 10.1 implementation
let handler = Update::filter_message().endpoint(
    move |msg: Message, bot: Bot| {
        async move {
            let chat_id = msg.chat.id.to_string();
            let content = msg.text().unwrap_or("").to_string();
            
            // ADD: Extract and check user_id here
            if let Some(user) = &msg.from {
                let user_id = user.id.0 as i64;
                // Check whitelist before processing
            }
            
            // ... rest of message processing
        }
    }
);
```

**From Story 2.2 (Interactive Onboarding):**

- Uses `inquire` crate for interactive prompts
- Config saved to `~/.miniclaw/config.json`
- Pattern: prompt → validate → store → confirm
- User ID explanation should be clear and actionable

**Git Intelligence - Recent Patterns:**

```
920eb74 Add telegram bot adapter
699f736 Update dependencies for Telegram integration
```

- Recent focus on Telegram channel implementation
- Security considerations being added alongside features
- Pattern: Implement feature → Add tests → Update integration points

**Security Pattern from Epic 6:**

```rust
// From exec tool blacklist (NFR-S4)
const BLACKLISTED_COMMANDS: &[&str] = &[
    "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo"
];

pub fn is_command_allowed(command: &str) -> bool {
    !BLACKLISTED_COMMANDS.contains(&command.to_lowercase().as_str())
}
```

Similar pattern applies: check against list → allow/reject → log appropriately

### Anti-Pattern Prevention

**CRITICAL: Avoid these common mistakes:**

1. **Don't allow all by default** - Empty whitelist MUST reject all (secure by default per NFR-S5)
2. **Don't fail silently without logging** - Always log rejected messages at DEBUG level
3. **Don't expose whitelist contents in logs** - Log only that a user was rejected, not the full list
4. **Don't use i32 for user IDs** - Telegram user IDs can exceed i32 range, use i64
5. **Don't forget to update gateway.rs** - Must pass allow_from when creating TelegramChannel
6. **Don't break existing Telegram tests** - Whitelist should be optional in tests (use empty vec)
7. **Don't store whitelist in session** - It's config, not session state
8. **Don't allow negative user IDs** - Validate all IDs are positive integers

**Security Best Practices:**

- **Secure by Default**: Empty whitelist = no access (NFR-S5 compliance)
- **Fail Closed**: Any error in whitelist check should reject the message
- **Audit Trail**: Log all whitelist decisions (allowed at DEBUG, rejected at DEBUG, empty at WARN)
- **Immutable Lists**: Consider whitelist immutable after initialization (Arc<Vec<i64>>)
- **Input Validation**: Validate user IDs are positive before adding to config

**Performance Considerations:**

- Whitelist lookup should be O(n) or better (n = small number of allowed users)
- Use HashSet<i64> if whitelist grows large (unlikely for personal agent)
- Whitelist check happens BEFORE any message processing to minimize resource usage

### References

- **NFR-S5**: Security requirement for whitelist [Source: _bmad-output/planning-artifacts/epics.md#NonFunctional-Requirements]
- **FR36**: Functional requirement for whitelist filtering [Source: _bmad-output/planning-artifacts/epics.md#Functional-Requirements]
- **Story 10.1**: Telegram Bot Adapter implementation [Source: _bmad-output/implementation-artifacts/10-1-telegram-bot-adapter.md]
- **Story 2.2**: Onboarding pattern [Source: _bmad-output/implementation-artifacts/2-2-interactive-onboarding-command.md]
- **Architecture Security**: Utils security module [Source: _bmad-output/planning-artifacts/architecture.md#Project-Structure-&-Boundaries]
- **Epic 10**: Telegram Channel Integration [Source: _bmad-output/planning-artifacts/epics.md#Epic-10-Telegram-Channel-Integration]
- **PRD Section**: Telegram configuration [Source: _bmad-output/planning-artifacts/prd.md#Configuration]

## Latest Technical Information

**Telegram User ID Characteristics:**
- Type: i64 (signed 64-bit integer)
- Range: Positive integers, typically 5-10 digits
- Uniqueness: Globally unique across all Telegram users
- Stability: Permanent, never changes for a user

**Finding User ID:**
- Bot: @userinfobot - Send /start, bot replies with user ID
- Web: https://t.me/userinfobot
- Alternative: @raw_data_bot, @getidsbot

**Config File Security:**
- Location: `~/.miniclaw/config.json`
- Permissions: Already set to 0600 by Story 2.1
- User IDs are not sensitive (unlike tokens), but config file is protected anyway

**Integration with Existing Code:**
- TelegramChannel::new() signature needs to change to accept whitelist
- All existing tests will need to be updated with empty whitelist parameter
- Gateway initialization needs to pass config.allow_from to TelegramChannel

## Dev Agent Record

### Agent Model Used

kimi-for-coding/k2p5 (Claude Code)

### Debug Log References

- [2026-02-16] Created src/utils/security.rs with WhitelistChecker struct
- [2026-02-16] Updated Config schema: telegram_whitelist → allow_from (Vec<i64>)
- [2026-02-16] Integrated whitelist checking in TelegramChannel::new() and message handler
- [2026-02-16] Updated onboarding flow with French prompts for whitelist configuration
- [2026-02-16] Updated gateway.rs to pass config.allow_from to TelegramChannel
- [2026-02-16] Updated config/loader.rs to parse MINICLAW_ALLOW_FROM env variable
- [2026-02-16] Fixed all references to old telegram_whitelist field across codebase
- [2026-02-16] All tests passing: 589 tests total

### Completion Notes List

1. **Security Module Created (AC: 1-5)**
   - Created `src/utils/security.rs` with WhitelistChecker using HashSet<i64> for O(1) lookups
   - Implemented secure-by-default: empty whitelist rejects all (NFR-S5 compliance)
   - Added proper logging: WARN for empty whitelist, DEBUG for rejected users
   - Added command blacklist from Epic 6 for consistency

2. **Config Schema Updated (AC: 1)**
   - Changed `telegram_whitelist: Option<Vec<String>>` to `allow_from: Vec<i64>`
   - Added #[serde(default)] for backward compatibility
   - Added validation to reject non-positive user IDs
   - Updated all tests and references across codebase

3. **Telegram Adapter Integration (AC: 2-5)**
   - Updated TelegramChannel::new() to accept whitelist Vec<i64>
   - Added whitelist check before processing messages in dispatcher
   - Extracts user_id from msg.from.id (u64 cast to i64)
   - Silently drops messages from non-whitelisted users with DEBUG log
   - Warns when whitelist is empty

4. **Onboarding Updated (AC: 6)**
   - Added French language prompts for whitelist configuration
   - Explains how to find user ID via @userinfobot
   - Validates user ID is positive integer
   - Shows warning if no ID provided (empty whitelist = secure by default)

5. **Environment Variable Support**
   - Updated MINICLAW_ALLOW_FROM to parse comma-separated i64 values
   - Filters out invalid values (non-numeric, zero, negative)

6. **All Tests Passing**
   - 589 tests passing including new security tests
   - No regressions introduced

### File List

**New Files:**
- `src/utils/security.rs` - WhitelistChecker and security utilities (134 lines)

**Modified Files:**
- `src/utils/mod.rs` - Added security module export (1 line added)
- `src/config/schema.rs` - Changed telegram_whitelist to allow_from (Vec<i64>) with tests (77 lines)
- `src/config/loader.rs` - Updated to handle allow_from and MINICLAW_ALLOW_FROM env var (378 lines)
- `src/config/onboard.rs` - Updated whitelist prompt and configuration flow (442 lines)
- `src/channels/telegram.rs` - Integrated whitelist checking in message handler (367 lines)
- `src/gateway.rs` - Pass config.allow_from to TelegramChannel::new() (429 lines)
- `src/agent/oneshot.rs` - Fixed Config struct references (5 occurrences)
- `tests/telegram_tests.rs` - Updated test fixtures to use allow_from instead of telegram_whitelist (9 occurrences)

---

## Change Log

**2026-02-16:** Story 10.2 - Telegram Whitelist - Story Created
- Comprehensive story context created with full architecture compliance
- Acceptance Criteria defined from Epic 10 requirements
- Technical implementation patterns documented
- Dependencies on Story 10.1 identified
- Security requirements (NFR-S5) fully addressed
- File structure and integration points specified
- Previous story intelligence from 10.1 incorporated
- Status: backlog → ready-for-dev

**2026-02-16:** Story 10.2 - Implementation Complete
- ✅ Created WhitelistChecker in src/utils/security.rs with secure-by-default logic
- ✅ Updated Config schema: telegram_whitelist → allow_from (Vec<i64>)
- ✅ Integrated whitelist checking in TelegramChannel message handler
- ✅ Updated onboarding with French prompts for whitelist configuration
- ✅ Added environment variable MINICLAW_ALLOW_FROM support
- ✅ Fixed all codebase references to use new allow_from field
- ✅ All 589 tests passing, no regressions
- ✅ Full NFR-S5 compliance: empty whitelist = no access
- Status: ready-for-dev → in-progress → review

**2026-02-16:** Story 10.2 - Code Review Complete (AI-Review)
- **Reviewer:** Claude (code-review workflow)
- **Issues Found:** 3 HIGH, 2 MEDIUM, 2 LOW
- **Issues Fixed:** 5 (all HIGH and MEDIUM issues)
- **Fixes Applied:**
  1. ✅ Updated File List to include tests/telegram_tests.rs (missing from documentation)
  2. ✅ Removed duplicate logging in WhitelistChecker.is_allowed() and Telegram handler
  3. ✅ Added 4 missing integration tests for whitelist functionality (AC 2-5)
  4. ✅ Fixed WARN logging frequency by moving to constructor (one-time warning)
  5. ✅ Added validation to WhitelistChecker.add_user() to reject invalid IDs
- **Test Results:** All 592 tests passing (added 3 new tests)
- **Outstanding:** 2 LOW priority issues documented but not blocking
- Status: review → done
