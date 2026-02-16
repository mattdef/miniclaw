---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - prd.md
  - product-brief-miniclaw-2026-02-14.md
  - docs/PLAN_PROJECT.md
workflowType: 'architecture'
project_name: 'miniclaw'
user_name: 'Matt'
date: '2026-02-14'
lastStep: 8
status: 'complete'
completedAt: '2026-02-14'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
47 FRs organisés en 6 domaines fonctionnels. Le cœur architectural comprend : Chat Hub (channels tokio mpsc), Agent Loop (pattern Receive→Context→LLM→Tools→Reply), Context Builder (assemblage 8 couches), 11 outils, Memory System (short-term/long-term/daily), Session Manager, Cron Scheduler, et Telegram Channel.

**Non-Functional Requirements:**
- Performance stricte : <15MB binaire, <30MB RAM, <100ms cold start, <2s réponse
- Sécurité : Path canonicalization, exec blacklist, permissions 0600, HTTPS/TLS 1.2+
- Fiabilité : Auto-restart, persistence 30s, gestion gracieuse erreurs
- Compatibilité : Linux ARM64/AMD64, Windows x86-64, zero runtime dependencies

**Scale & Complexity:**
- Primary domain: CLI tool / Agent IA edge autonome
- Complexity level: Moyenne-Élevée
- Estimated architectural components: 10+ modules core (config, chat, agent, tools, memory, channels, session, cron, providers)

### Technical Constraints & Dependencies
- Rust 1.85+ Edition 2024 obligatoire
- Zero database (JSON/markdown files uniquement)
- Hardware cible : Raspberry Pi 3 (1GB RAM) minimum
- Stack : tokio, reqwest, serde, clap, chrono, regex, anyhow/thiserror, tracing, teloxide
- Providers : OpenAI-compatible (OpenRouter), Ollama local

### Cross-Cutting Concerns Identified
1. **Memory Safety** : Zero unsafe code, ownership clair
2. **Async Concurrency** : tokio mpsc channels, RwLock/Mutex pour sessions
3. **Security Sandboxing** : Filesystem path validation, exec restrictions
4. **Resource Constraints** : Optimisation pour 256MB RAM
5. **Extensibility** : Plugin system skills, trait-based tools/providers

## Starter Template Evaluation

### Primary Technology Domain
CLI Tool / Agent IA autonome écrit en Rust, fonctionnant en mode interactif et daemon.

### Approach Selected: Modular Rust Architecture

Pour un projet Rust CLI/agent de cette nature, il n'existe pas de "starter template" conventionnel comme dans l'écosystème JavaScript. L'approche choisie est une architecture modulaire standard Rust avec les caractéristiques suivantes :

**Structure du projet:**
- Single crate avec modules hiérarchiques (pas de workspace complexe)
- Organisation par domaine fonctionnel (config/, chat/, agent/, tools/, memory/, channels/, session/, cron/, providers/)

**Configuration de build:**
- Edition 2024, Rust 1.85+
- Profil release optimisé pour la taille binaire (strip, LTO potentiel en Growth)
- Zero unsafe code policy

**Patterns architecturaux établis:**
- Traits pour extensibilité (LLMProvider, Tool, Channel)
- Registry pattern pour les outils
- Builder pattern pour le ContextBuilder
- Async/await avec tokio pour toutes les I/O

**Note:** La première story d'implémentation sera `cargo init` avec la structure de modules définie.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Architecture Data & Persistence (sérialisation, concurrence, rotation)
- Gestion des erreurs (anyhow vs thiserror)
- Patterns de concurrence (sessions, Agent Loop)

**Important Decisions (Shape Architecture):**
- Configuration de build Rust (optimisations Release)
- Structure des modules et dépendances
- Patterns de logging et observabilité

**Deferred Decisions (Post-MVP):**
- Optimisations avancées Rust (LTO, strip)
- Format binaire alternatif (MessagePack si JSON insuffisant)
- Database externe (si scale dépasse fichiers)

### Data Architecture

**1.1 Sérialisation : JSON pur (serde_json)**
- **Choix** : JSON pur avec serde_json
- **Rationale** : Lisibilité humaine pour debug, standard écosystème Rust, suffisant pour MVP avec <50 messages/session
- **Version** : serde_json latest stable
- **Affects** : Sessions persistence, configuration, mémoire
- **Implique** : Pas de dépendance MessagePack, fichiers sessions lisibles pour debug

**1.2 Stratégie de concurrence fichier : Arc<RwLock<HashMap>>**
- **Choix** : Verrouillage en mémoire avec Arc<RwLock<HashMap>> par session
- **Rationale** : Performance optimale, tokio-native, sessions isolées par chat_id, pas de conflits cross-user
- **Affects** : SessionManager, Gateway daemon
- **Implique** : Task de background pour persistence auto toutes les 30s
- **Pattern** : tokio::spawn(persistence_loop()) dans le gateway

**1.3 Rotation des sessions : TTL 30j avec sliding window**
- **Choix** : TTL 30 jours avec actualisation automatique des sessions actives
- **Rationale** : Sessions actives préservées automatiquement, nettoyage automatique des sessions abandonnées
- **Affects** : SessionManager, struct Session (champ last_accessed: DateTime<Utc>)
- **Implique** : Task cron quotidien pour cleanup, mise à jour last_accessed à chaque message
- **Seuil** : 30 jours d'inactivité = suppression

### Decision Impact Analysis

**Implementation Sequence:**
1. SessionManager avec Arc<RwLock<HashMap>>
2. Persistence loop (toutes les 30s)
3. Cleanup cron (quotidien)
4. Configuration serde_json

**Cross-Component Dependencies:**
- SessionManager → tokio::sync::RwLock
- Persistence → tokio::time::interval
- Cleanup → chrono::Utc pour date/heure
- JSON → serde + serde_json avec derive features

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 15+ areas where AI agents could make different choices

---

### Naming Patterns

**Rust Code Naming Conventions:**

| Element | Convention | Example | Rationale |
|---------|-----------|---------|-----------|
| Modules | snake_case | `chat_hub`, `context_builder` | Rust RFC 430 |
| Structs/Enums | PascalCase | `SessionManager`, `ToolError` | Rust RFC 430 |
| Traits | PascalCase (descriptive) | `LLMProvider`, `Tool`, `Channel` | Express intent clearly |
| Functions/Methods | snake_case | `get_user_data()`, `execute_tool()` | Rust RFC 430 |
| Variables | snake_case | `user_id`, `chat_message` | Rust RFC 430 |
| Constants | SCREAMING_SNAKE_CASE | `MAX_ITERATIONS`, `BUFFER_SIZE` | Rust RFC 430 |
| Type aliases | PascalCase | `SessionId`, `ToolResult` | Consistency with types |
| Generic params | Single uppercase | `T`, `E`, `K`, `V` | Rust convention |

**File Naming Conventions:**

| Type | Pattern | Example |
|------|---------|---------|
| Module files | snake_case.rs | `chat_hub.rs`, `agent_loop.rs` |
| Module folders | snake_case/ | `tools/`, `memory/` |
| Test files | module_name_test.rs OR co-located `#[cfg(test)]` | Prefer co-located in `mod.rs` |
| Config files | snake_case.json/.md | `config.json`, `memory.md` |
| Session files | {channel}_{chat_id}.json | `telegram_123456789.json` |

**Error Type Naming:**
- Error enums: `{Module}Error` → `AgentError`, `ToolError`, `ConfigError`
- Error variants: descriptive with context → `InvalidPath { path: String }`
- Result aliases: `type Result<T> = std::result::Result<T, ModuleError>;`

---

### Structure Patterns

**Project Organization:**

```
src/
├── main.rs              # CLI entry (clap)
├── lib.rs               # Library exports (optional)
├── config/
│   ├── mod.rs           # Config module
│   ├── schema.rs        # Config structs
│   └── onboard.rs       # Workspace init
├── chat/
│   ├── mod.rs           # ChatHub (mpsc channels)
│   └── types.rs         # InboundMessage, OutboundMessage
├── agent/
│   ├── mod.rs           # AgentLoop
│   ├── context.rs       # ContextBuilder
│   ├── loop.rs          # Main loop logic
│   └── types.rs         # Agent types
├── tools/
│   ├── mod.rs           # Tool trait & Registry
│   ├── filesystem.rs    # filesystem tool
│   ├── exec.rs          # exec tool
│   ├── web.rs           # web tool
│   ├── message.rs       # message tool
│   ├── spawn.rs         # spawn tool
│   ├── cron.rs          # cron tool
│   ├── memory.rs        # memory tool
│   └── skill.rs         # skill tools
├── memory/
│   ├── mod.rs           # MemoryStore
│   ├── short_term.rs    # VecDeque memory
│   ├── long_term.rs     # MEMORY.md management
│   └── ranker.rs        # Simple ranker
├── channels/
│   ├── mod.rs           # Channel trait
│   └── telegram.rs      # Telegram adapter
├── session/
│   ├── mod.rs           # SessionManager
│   └── types.rs         # Session struct
├── cron/
│   ├── mod.rs           # Cron scheduler
│   └── types.rs         # Job types
└── providers/
    ├── mod.rs           # LLMProvider trait
    └── openai.rs        # OpenAI-compatible
```

**File Organization Rules:**
1. **One module per domain**: Each folder is a domain (chat/, agent/, tools/)
2. **mod.rs is the interface**: Public exports only in mod.rs
3. **Implementation in named files**: Private modules for implementation details
4. **types.rs for shared types**: When multiple files need the same types
5. **Tests co-located**: `#[cfg(test)]` module at bottom of each file

**Test Organization:**
```rust
// In src/agent/mod.rs
pub struct AgentLoop { ... }

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_loop_basic() {
        // Test implementation
    }
}
```

---

### Format Patterns

**JSON Response/Session Formats:**

```json
// Session file: telegram_123456789.json
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
    }
  ]
}
```

**Field Naming Convention:** snake_case everywhere (JSON, Rust, files)

**Date/Time Format:**
- **Always ISO 8601 with UTC**: `2026-02-14T15:45:00Z`
- **Type**: `chrono::DateTime<chrono::Utc>`
- **Serialization**: `#[serde(with = "chrono::serde::ts_seconds")]` or RFC3339 string

**Error Response Format:**
```rust
// Tool execution error
pub struct ToolError {
    pub tool_name: String,
    pub error_type: ErrorType,  // enum
    pub message: String,
    pub recoverable: bool,
}

pub enum ErrorType {
    InvalidArguments,
    ExecutionFailed,
    PermissionDenied,
    Timeout,
    NotFound,
}
```

**API Result Pattern:**
```rust
// Consistent Result type across modules
pub type Result<T> = std::result::Result<T, ModuleError>;

// Error propagation
pub async fn execute(&self, args: Args) -> Result<String> {
    let validated = self.validate(args).map_err(|e| ModuleError::Validation(e))?;
    // ...
}
```

---

### Communication Patterns

**Channel Communication (mpsc):**
```rust
// Inbound messages from channels
pub struct InboundMessage {
    pub channel: String,        // "telegram"
    pub chat_id: String,        // user identifier
    pub content: String,        // message text
    pub metadata: HashMap<String, Value>,
}

// Outbound messages to channels  
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>, // message_id to reply to
}
```

**Event Naming:**
- No global event system (keep it simple)
- Direct method calls between components
- Async communication via mpsc channels only

**State Updates:**
- Immutable updates: Clone + modify → replace in HashMap
- Lock scope minimal: acquire lock, clone data, release lock, process, re-acquire, update

---

### Process Patterns

**Error Handling:**

**Rule 1: Use `?` for propagation**
```rust
// GOOD
let config = load_config().await?;

// BAD (unless specific error handling needed)
let config = match load_config().await {
    Ok(c) => c,
    Err(e) => return Err(e.into()),
};
```

**Rule 2: Convert errors at boundaries**
```rust
// Convert external errors to domain errors
impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::ParseError(e.to_string())
    }
}
```

**Rule 3: Context with anyhow (CLI) / thiserror (library)**
- CLI/main: `anyhow` for easy context
- Library code: `thiserror` for typed errors

**Async Patterns:**

**Rule 1: Async everywhere I/O**
```rust
// All I/O operations must be async
pub async fn load_session(&self, id: &str) -> Result<Session>;
pub async fn save_memory(&self, content: &str) -> Result<()>;
```

**Rule 2: Spawn for background tasks**
```rust
// Persistence loop
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = persistence.save_all().await {
            tracing::error!("Persistence failed: {}", e);
        }
    }
});
```

**Rule 3: Select! for multiple async sources**
```rust
loop {
    tokio::select! {
        Some(msg) = inbound_rx.recv() => {
            self.handle_message(msg).await;
        }
        _ = shutdown.recv() => {
            tracing::info!("Shutting down gracefully...");
            break;
        }
    }
}
```

**Logging with tracing:**

**Levels:**
- `ERROR`: Failures requiring intervention (crash, data loss risk)
- `WARN`: Unexpected but handled (rate limit, retry)
- `INFO`: Important lifecycle events (startup, config loaded, message received)
- `DEBUG`: Detailed operation info (tool execution, context assembly)
- `TRACE`: Very verbose (serialization, raw API responses)

**Format:**
```rust
// Structured logging with fields
tracing::info!(chat_id = %chat_id, channel = %channel, "Message received");
tracing::debug!(tool = %tool_name, args = ?args, "Executing tool");
tracing::error!(error = %e, "Failed to load configuration");
```

**Never log secrets:**
```rust
// BAD
tracing::info!("API key: {}", api_key);

// GOOD
tracing::info!("API key configured: {}", !api_key.is_empty());
```

---

### Enforcement Guidelines

**All AI Agents MUST:**
1. Follow Rust RFC 430 naming conventions strictly
2. Use snake_case for all JSON field names
3. Store dates as ISO 8601 UTC strings
4. Keep tests co-located in `#[cfg(test)]` modules
5. Use `?` operator for error propagation (don't match unless needed)
6. Make all I/O operations async
7. Use tracing with structured fields (not println!)
8. Never log secrets, tokens, or API keys
9. Use Arc<RwLock<T>> for shared mutable state
10. Keep lock scopes minimal (clone data, release lock, process)

**Pattern Verification:**
- **Linting**: `cargo clippy` with strict warnings
- **Formatting**: `cargo fmt` with default config
- **Review**: Check for consistency in PR reviews

**Anti-Patterns to AVOID:**
```rust
// ❌ DON'T: Mixed naming conventions
struct userData { user_id: i32 }  // Mixed PascalCase and snake_case

// ❌ DON'T: Unstructured logging
println!("Got message: {}", msg);

// ❌ DON'T: Blocking in async
std::fs::read_to_string(path)  // Use tokio::fs instead

// ❌ DON'T: Panic on user input
.unwrap()  // Use ? or match with proper error handling

// ❌ DON'T: Long lock holding
let data = mutex.lock().await;
// ... lots of processing while holding lock ...
drop(data);

// ✅ DO: Clone and release quickly
let data = mutex.lock().await.clone();
drop(lock);
// ... process data without lock ...
```

**Good Examples:**
```rust
// ✅ Consistent naming and error handling
pub async fn execute_tool(&self, name: &str, args: Value) -> ToolResult<String> {
    let tool = self.registry.get(name).ok_or(ToolError::NotFound)?;
    let result = tool.execute(args).await?;
    tracing::info!(tool = %name, "Tool executed successfully");
    Ok(result)
}

// ✅ Proper async file I/O
use tokio::fs;
let content = fs::read_to_string(path).await?;
```

## Project Structure & Boundaries

### Complete Project Directory Structure

```
miniclaw/
├── Cargo.toml                    # Package manifest
├── Cargo.lock                    # Dependency lock (committed)
├── rustfmt.toml                  # Formatting config (optional)
├── clippy.toml                   # Linting config (optional)
├── .gitignore                    # Git ignore patterns
├── .env.example                  # Example environment variables
├── README.md                     # Project documentation
├── LICENSE                       # Apache-2.0 license
├── CHANGELOG.md                  # Version history
├── CONTRIBUTING.md               # Contribution guidelines
├── .github/
│   └── workflows/
│       ├── ci.yml                # CI: test, clippy, fmt
│       └── release.yml           # Release builds
├── src/
│   ├── main.rs                   # CLI entry point (clap)
│   ├── lib.rs                    # Library exports (optional)
│   ├── config/
│   │   ├── mod.rs                # Public Config API
│   │   ├── schema.rs             # Config structs (JSON)
│   │   ├── loader.rs             # Config file loading
│   │   └── onboard.rs            # Workspace initialization
│   ├── chat/
│   │   ├── mod.rs                # ChatHub (mpsc channels)
│   │   └── types.rs              # InboundMessage, OutboundMessage
│   ├── agent/
│   │   ├── mod.rs                # Public Agent API
│   │   ├── loop.rs               # AgentLoop implementation
│   │   ├── context.rs            # ContextBuilder
│   │   ├── types.rs              # Agent types & state
│   │   └── tools/                # Tool trait & implementations
│   │       ├── mod.rs            # Tool trait & Registry
│   │       ├── filesystem.rs     # filesystem tool
│   │       ├── exec.rs           # exec tool
│   │       ├── web.rs            # web tool
│   │       ├── message.rs        # message tool
│   │       ├── spawn.rs          # spawn tool
│   │       ├── cron.rs           # cron tool
│   │       ├── memory.rs         # memory tool
│   │       └── skill.rs          # skill tools (create/read/list/delete)
│   ├── memory/
│   │   ├── mod.rs                # MemoryStore public API
│   │   ├── short_term.rs         # VecDeque in-memory storage
│   │   ├── long_term.rs          # MEMORY.md management
│   │   ├── daily_notes.rs        # YYYY-MM-DD.md files
│   │   └── ranker.rs             # Simple keyword ranker
│   ├── channels/
│   │   ├── mod.rs                # Channel trait definition
│   │   └── telegram.rs           # Telegram bot adapter
│   ├── session/
│   │   ├── mod.rs                # SessionManager public API
│   │   ├── manager.rs            # Session management logic
│   │   ├── persistence.rs        # Auto-save every 30s
│   │   ├── cleanup.rs            # TTL cleanup task
│   │   └── types.rs              # Session struct & types
│   ├── cron/
│   │   ├── mod.rs                # Cron scheduler public API
│   │   ├── scheduler.rs          # In-memory scheduler
│   │   └── types.rs              # Job types (FireAt, Interval)
│   ├── providers/
│   │   ├── mod.rs                # LLMProvider trait
│   │   └── openai.rs             # OpenAI-compatible provider
│   └── utils/                    # Shared utilities
│       ├── mod.rs
│       ├── paths.rs              # Path validation & canonicalization
│       └── security.rs           # Security helpers (exec blacklist, etc.)
├── tests/                        # Integration tests
│   ├── integration/
│   │   ├── agent_test.rs
│   │   ├── tools_test.rs
│   │   └── session_test.rs
│   └── fixtures/                 # Test data
│       ├── config/
│       └── sessions/
├── benches/                      # Performance benchmarks
│   └── agent_loop_bench.rs
├── docs/                         # Additional documentation
│   ├── ARCHITECTURE.md           # This document reference
│   ├── TOOLS.md                  # Tool documentation
│   ├── SETUP.md                  # Setup guide
│   └── API.md                    # Provider API docs
└── scripts/                      # Helper scripts
    ├── install.sh                # Installation script
    └── release.sh                # Release build script
```

### Workspace utilisateur (créé par `miniclaw onboard`)

```
~/.miniclaw/
├── config.json                   # User configuration
├── workspace/
│   ├── SOUL.md                   # Agent personality
│   ├── AGENTS.md                 # Agent instructions
│   ├── USER.md                   # User profile
│   ├── TOOLS.md                  # Tool documentation
│   ├── HEARTBEAT.md              # Periodic tasks
│   ├── memory/
│   │   ├── MEMORY.md             # Long-term memory
│   │   ├── 2026-02-14.md         # Daily notes
│   │   └── 2026-02-13.md
│   ├── sessions/                 # Session persistence
│   │   ├── telegram_123456789.json
│   │   └── telegram_987654321.json
│   └── skills/                   # User-created skills
│       ├── weather/
│       │   └── SKILL.md
│       └── reminder/
│           └── SKILL.md
└── logs/                         # Application logs
    └── miniclaw.log
```

### Requirements to Structure Mapping

| FR/Module | Fichier(s) principal(aux) | Responsabilité |
|-----------|---------------------------|----------------|
| **FR1-11: CLI** | `src/main.rs`, `src/config/` | Command parsing, onboarding |
| **FR12-17: Agent** | `src/agent/` | Loop, context, tool execution |
| **FR18-23: Memory** | `src/memory/` | Short/long term, ranker |
| **FR24-34: Tools** | `src/agent/tools/` | 11 tool implementations |
| **FR35-38: Channels** | `src/channels/` | Telegram adapter |
| **FR39-44: Config** | `src/config/`, `~/.miniclaw/` | JSON config, workspace |
| **FR45-47: Logging** | `src/main.rs` (tracing setup) | Structured logging |
| **NFR-P1-5: Performance** | `Cargo.toml` (optimizations) | Binary size, RAM, speed |
| **NFR-S1-6: Security** | `src/utils/security.rs`, `src/agent/tools/exec.rs` | Validation, sandboxing |
| **NFR-R1-5: Reliability** | `src/session/persistence.rs` | Auto-save, graceful shutdown |
| **NFR-C1-5: Compatibility** | `Cargo.toml`, CI | Cross-platform builds |

### Architectural Boundaries

**API Boundaries (Traits):**
```rust
// src/providers/mod.rs - LLMProvider boundary
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, model: &str) -> Result<LLMResponse>;
    fn default_model(&self) -> String;
}

// src/agent/tools/mod.rs - Tool boundary  
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    async fn execute(&self, args: HashMap<String, Value>, ctx: &ToolExecutionContext) -> ToolResult<String>;
}

// src/channels/mod.rs - Channel boundary
pub trait Channel: Send + Sync {
    async fn start(&self, hub: Arc<ChatHub>) -> Result<()>;
    async fn send(&self, message: OutboundMessage) -> Result<()>;
}
```

**Component Boundaries:**
- **ChatHub** (`src/chat/`): Point central de communication via mpsc channels
- **AgentLoop** (`src/agent/`): Orchestration Receive→Context→LLM→Tools→Reply
- **SessionManager** (`src/session/`): Isolation des sessions avec RwLock
- **ToolRegistry** (`src/agent/tools/mod.rs`): Découverte dynamique des outils

**Data Flow:**
```
Telegram → InboundMessage → ChatHub.inbound_rx → AgentLoop → ContextBuilder → LLMProvider
                                                                    ↓
User ← OutboundMessage ← ChatHub.outbound_tx ← Tool Execution ← Tool Calls
```

**Security Boundaries:**
- **Filesystem**: `src/utils/paths.rs` - canonicalize() + validation
- **Exec**: `src/agent/tools/exec.rs` - blacklist + array args only
- **Sessions**: File permissions 0600 sur config.json

### Integration Points

**Internal Communication:**
- ChatHub mpsc channels (tokio::sync::mpsc) entre tous les composants
- Direct method calls pour les opérations synchrones
- Traits pour extensibilité (LLMProvider, Tool, Channel)

**External Integrations:**
- **Telegram API**: teloxide crate via HTTPS
- **LLM Providers**: reqwest with JSON over HTTPS
- **Filesystem**: tokio::fs pour toutes les opérations fichiers
- **Ollama**: HTTP localhost pour LLM local

**Data Persistence:**
- Sessions: JSON files in `~/.miniclaw/workspace/sessions/`
- Memory: Markdown files in `~/.miniclaw/workspace/memory/`
- Config: JSON file in `~/.miniclaw/config.json`
- Auto-save every 30s via tokio::spawn background task

### File Organization Patterns

**Configuration Files:**
- `Cargo.toml`: Package metadata, dependencies, build profiles
- `.env.example`: Template for environment variables (secrets)
- `config.json`: Runtime configuration (structural, non-secret)

**Source Organization:**
- Public API in `mod.rs` files only
- Implementation details in named modules
- Shared types in `types.rs` when needed by multiple files
- Utils for cross-cutting concerns (paths, security)

**Test Organization:**
- Unit tests co-located in `#[cfg(test)]` modules
- Integration tests in `tests/integration/`
- Fixtures in `tests/fixtures/`
- Benchmarks in `benches/`

**Asset Organization:**
- Documentation in `docs/`
- Scripts in `scripts/`
- CI/CD in `.github/workflows/`

### Development Workflow Integration

**Development Server Structure:**
```bash
# Development
cargo run -- agent -m "test message"
cargo run -- gateway

# Testing
cargo test
cargo test --lib
cargo test --test integration

# Benchmarking
cargo bench

# Linting & Formatting
cargo clippy -- -D warnings
cargo fmt -- --check
```

**Build Process Structure:**
```toml
# Cargo.toml optimization for size
[profile.release]
opt-level = 3
lto = true        # Enable in Growth phase if needed
strip = true      # Strip symbols
panic = 'abort'   # Smaller binary
```

**Deployment Structure:**
- Binary distribution: `cargo build --release`
- Docker: Alpine Linux base for minimal image
- Cross-compilation: targets for ARM64 (Raspberry Pi) and AMD64

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
All architectural decisions work together harmoniously:
- Rust 1.85+ Edition 2024 is fully compatible with tokio, serde, clap, and the entire async ecosystem
- JSON serialization with serde_json aligns perfectly with persistence requirements and debug needs
- Arc<RwLock<HashMap>> with tokio::spawn provides the optimal balance of performance and safety for session management
- Trait-based architecture (LLMProvider, Tool, Channel) enables the required extensibility without compromising type safety

**Pattern Consistency:**
All implementation patterns support and reinforce the architectural decisions:
- Rust RFC 430 naming conventions applied consistently across all modules, files, and APIs
- snake_case convention maintained throughout (Rust code, JSON fields, file names)
- Modular structure directly supports the 10+ identified architectural components
- Async patterns (tokio::select!, spawn) align with the chosen concurrency model

**Structure Alignment:**
The project structure fully supports all architectural requirements:
- Directory hierarchy maps 1:1 with functional requirements (FR categories)
- Component boundaries clearly defined via traits and module visibility
- Integration points (mpsc channels) properly structured for loose coupling
- Security boundaries isolated in dedicated utils modules

### Requirements Coverage Validation ✅

**Functional Requirements Coverage:**
All 47 FRs are architecturally supported:

| Category | Count | Status | Architecture Support |
|----------|-------|--------|---------------------|
| CLI Interface (FR1-11) | 11 | ✅ | src/main.rs, src/config/ |
| Agent Conversationnel (FR12-17) | 6 | ✅ | src/agent/ |
| Memory System (FR18-23) | 6 | ✅ | src/memory/ |
| Tools & Capabilities (FR24-34) | 11 | ✅ | src/agent/tools/ |
| Channel Management (FR35-38) | 4 | ✅ | src/channels/ |
| Configuration (FR39-44) | 6 | ✅ | src/config/, ~/.miniclaw/ |
| Logging (FR45-47) | 3 | ✅ | tracing infrastructure |

**Non-Functional Requirements Coverage:**
All NFRs addressed architecturally:

| Category | Requirements | Status | Implementation |
|----------|-------------|--------|----------------|
| Performance (NFR-P1-5) | Binary <15MB, RAM <30MB, startup <100ms | ✅ | Cargo.toml optimizations, efficient data structures |
| Security (NFR-S1-6) | Secrets, path validation, exec restrictions | ✅ | utils/security.rs, tools/exec.rs |
| Reliability (NFR-R1-5) | Persistence, error handling, graceful shutdown | ✅ | Session persistence every 30s, SIGTERM handling |
| Compatibility (NFR-C1-5) | ARM64, AMD64, zero dependencies | ✅ | Cross-platform targets, static linking |

### Implementation Readiness Validation ✅

**Decision Completeness:**
- ✅ All critical decisions documented with specific technology choices and versions
- ✅ Implementation patterns comprehensive with concrete code examples
- ✅ Consistency rules clear, numbered, and enforceable via linting
- ✅ Examples provided for all major patterns (error handling, async, logging)

**Structure Completeness:**
- ✅ Complete directory structure defined with all files and subdirectories
- ✅ Every component mapped to specific source files
- ✅ Integration points explicitly specified (ChatHub mpsc channels)
- ✅ Component boundaries defined via public traits

**Pattern Completeness:**
- ✅ 15+ potential conflict points identified and addressed
- ✅ Naming conventions comprehensive (modules, types, functions, variables, constants)
- ✅ Communication patterns fully specified (mpsc channels, direct calls, traits)
- ✅ Process patterns complete (error handling, async/await, logging levels)

### Gap Analysis Results

**Critical Gaps:**
🔴 **None identified** - All necessary architectural decisions are documented

**Important Gaps:**
🟡 Error handling mix: Documented to use anyhow in CLI/main, thiserror in library code
🟡 Cron persistence: Documented as in-memory only (no persistence required by PRD)

**Nice-to-Have Enhancements:**
🟢 Add architecture diagrams for data flow visualization
🟢 Expand CI/CD pipeline specifications
🟢 Include Docker configuration templates

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] Project context thoroughly analyzed (47 FRs, 19 NFRs)
- [x] Scale and complexity assessed (Medium-High, 10+ components)
- [x] Technical constraints identified (Rust 1.85+, Zero DB, 256MB RAM)
- [x] Cross-cutting concerns mapped (5 major concerns)

**✅ Architectural Decisions**
- [x] Critical decisions documented with versions (JSON, RwLock, TTL 30j)
- [x] Technology stack fully specified (tokio, serde, clap, tracing, teloxide)
- [x] Integration patterns defined (mpsc channels, trait boundaries)
- [x] Performance considerations addressed (release profile, optimizations)

**✅ Implementation Patterns**
- [x] Naming conventions established (RFC 430)
- [x] Structure patterns defined (modular hierarchy)
- [x] Communication patterns specified (async/await, channels)
- [x] Process patterns documented (error handling, logging)

**✅ Project Structure**
- [x] Complete directory structure defined (src/, tests/, docs/)
- [x] Component boundaries established (traits)
- [x] Integration points mapped (ChatHub, AgentLoop, SessionManager)
- [x] Requirements to structure mapping complete (FR→files table)

### Architecture Readiness Assessment

**Overall Status:** ✅ **READY FOR IMPLEMENTATION**

**Confidence Level:** **HIGH** 
- All decisions validated for coherence
- 100% requirements coverage verified
- No blocking gaps identified
- Comprehensive patterns with examples provided

**Key Strengths:**
1. **Clear separation of concerns** via modular architecture
2. **Type-safe extensibility** via trait-based design
3. **Performance-conscious** decisions for constrained hardware
4. **Security-first** approach with sandboxing and validation
5. **Async-native** design leveraging Rust's ownership system
6. **Zero-dependency runtime** for maximum compatibility

**Areas for Future Enhancement:**
1. Add architecture decision records (ADRs) for major choices
2. Create data flow diagrams for visual reference
3. Expand CI/CD specifications
4. Document benchmarking methodology

### Implementation Handoff

**AI Agent Guidelines:**

All AI agents implementing miniclaw MUST:

1. **Follow architectural decisions exactly** as documented - no deviations without discussion
2. **Use implementation patterns consistently** across all components and modules
3. **Respect project structure and boundaries** - place code in designated files
4. **Refer to this document** for all architectural questions and decisions
5. **Maintain zero unsafe code** policy throughout implementation
6. **Apply naming conventions strictly** per RFC 430
7. **Use Arc<RwLock<T>>** for all shared mutable state
8. **Implement async/await** for all I/O operations
9. **Use tracing** with structured fields for all logging
10. **Never log secrets** or sensitive configuration data

**First Implementation Priority:**

```bash
# Initialize Rust project
cargo init --name miniclaw
cd miniclaw

# Add core dependencies to Cargo.toml
cargo add tokio --features full
cargo add serde serde_json
cargo add clap --features derive
cargo add tracing tracing-subscriber
cargo add chrono --features serde
cargo add anyhow thiserror
cargo add reqwest --features json
cargo add teloxide
cargo add regex

# Create directory structure
mkdir -p src/{config,chat,agent/{tools,memory},channels,session,cron,providers,utils}
mkdir -p tests/integration tests/fixtures
mkdir -p benches docs scripts

# Start with config module ( foundational)
touch src/config/{mod,schema,loader,onboard}.rs
```

**Next Steps:**
1. Run `cargo init` and set up project structure
2. Implement config module first (required by all other modules)
3. Implement ChatHub (central communication)
4. Build AgentLoop with ContextBuilder
5. Add tools incrementally (filesystem, exec, web first)
6. Integrate Telegram channel
7. Add memory system
8. Implement session management with persistence
9. Add cron scheduler
10. Complete CLI commands

**Questions or Clarifications:**

If any AI agent encounters ambiguity during implementation:
- Check this document first for the specific decision or pattern
- If not documented, apply the closest matching pattern consistently
- Document the decision made for future reference
- Never guess or make assumptions that contradict documented patterns

---

## Workflow Completion Summary

**🎉 Félicitations Matt ! 🎉**

Nous avons ensemble créé une architecture complète et robuste pour **miniclaw** - votre agent IA autonome en Rust pour hardware edge.

### Ce que nous avons accompli :

✅ **Analyse approfondie** du contexte projet (47 FRs, 19 NFRs)
✅ **Décisions architecturales critiques** (sérialisation, concurrence, persistance)
✅ **Patterns d'implémentation** complets (naming, structure, communication)
✅ **Structure de projet** détaillée (arborescence complète avec tous les fichiers)
✅ **Validation exhaustive** (cohérence, couverture, préparation implémentation)

### Architecture livrée :

- **Single source of truth** pour toutes les décisions techniques
- **47 Functional Requirements** 100% couverts
- **19 Non-Functional Requirements** adressés
- **Zero blocking gaps** identifiés
- **Ready for implementation** par des agents IA

### Points forts de l'architecture :

🦀 **Rust idiomatique** - Edition 2024, RFC 430 conventions
⚡ **Performance optimisée** - <15MB binaire, <30MB RAM
🔒 **Sécurité intégrée** - Sandboxing, validation, zero unsafe
🔄 **Async natif** - tokio, channels, trait-based extensibility
💾 **Zero database** - Filesystem-only persistence

### Prochaines étapes suggérées :

1. **Initialiser le projet** : `cargo init` + structure modules
2. **Commencer par config** : Module fondamental requis par tous les autres
3. **Suivre les patterns** : Décisions architecturales dans ce document
4. **Implémenter itérativement** : Config → ChatHub → AgentLoop → Tools → Channels

Le document d'architecture est maintenant complet et sauvegardé dans :
`_bmad-output/planning-artifacts/architecture.md`

**Avez-vous des questions sur l'architecture documentée ?**
