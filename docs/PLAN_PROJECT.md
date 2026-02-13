# Création d'un Agent IA en Rust (type Picobot)

## Contexte & Objectif

Créer un agent IA autonome en Rust nommé MiniClaw - un binaire unique, léger (~5-10MB), fonctionnant sur hardware minimal (256MB RAM), avec les mêmes capacités que Picobot (en GO) mais en Rust.

## Architecture Core (à implémenter)

```
rustbot/
├── src/
│ ├── main.rs # CLI entry (clap)
│ ├── config/
│ │ ├── mod.rs # Configuration struct & loader
│ │ ├── schema.rs # Config types (JSON)
│ │ └── onboard.rs # Workspace initialization
│ ├── chat/
│ │ └── mod.rs # Message Hub (tokio mpsc channels)
│ ├── agent/
│ │ ├── mod.rs # AgentLoop struct & main loop
│ │ ├── context.rs # ContextBuilder (prompt assembly)
│ │ ├── tools/ # Tool implementations
│ │ │ ├── mod.rs # Tool trait & Registry
│ │ │ ├── filesystem.rs
│ │ │ ├── exec.rs
│ │ │ ├── web.rs
│ │ │ ├── message.rs
│ │ │ ├── cron.rs
│ │ │ ├── memory.rs
│ │ │ └── skill.rs
│ │ ├── memory/ # Memory system
│ │ │ ├── mod.rs # MemoryStore
│ │ │ └── ranker.rs # Simple + LLM ranking
│ │ └── skills/
│ │ └── mod.rs # Skills loader
│ ├── providers/
│ │ ├── mod.rs # LLMProvider trait
│ │ └── openai.rs # OpenAI-compatible provider
│ ├── channels/
│ │ └── telegram.rs # Telegram bot adapter
│ ├── session/
│ │ └── mod.rs # Session persistence
│ └── cron/
│ └── mod.rs # In-memory scheduler
```

## Spécifications Techniques

Stack obligatoire :

- Rust 1.80+
- Edition 2024

- Async runtime : tokio (full features)
- HTTP client : reqwest avec JSON support
- Serialization : serde + serde_json
- CLI : clap avec derive features
- Configuration : config crate ou JSON pur avec serde_json
- Dates : chrono
- Regex : regex crate
- Error handling : anyhow ou thiserror
- Logging : tracing + tracing-subscriber
- Telegram : teloxide

Traits fondamentaux à implémenter :

```rust
// Provider LLM #[async_trait]
trait LLMProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>, model: &str) -> Result<LLMResponse>;
    fn default_model(&self) -> String;
}

// Outils #[async_trait]
trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    async fn execute(&self, args: HashMap<String, Value>) -> Result<String>;
}

// Registry pattern pour les outils
struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}
```

## Fonctionnalités Requises

### 1. Chat Hub (chat/mod.rs)

- Utiliser tokio::sync::mpsc::channel pour Inbound/Outbound
- Types : InboundMessage, OutboundMessage
- Buffer size : 100 messages
- Thread-safe pour multi-channel

### 2. Agent Loop (agent/mod.rs)

- Boucle loop { select! { ... } } sur hub.inbound
- Max 200 itérations d'outils par message
- Pattern : Recevoir → Build Context → Call LLM → Execute Tools → Reply
- Gestion des tool calls en boucle jusqu'à réponse texte

### 3. Context Builder (agent/context.rs)

Assembler le prompt dans cet ordre :

1. System prompt : "You are MiniClaw..."
2. Bootstrap files (SOUL.md, AGENTS.md, USER.md, TOOLS.md) s'ils existent
3. Instruction mémoire : comment utiliser write_memory
4. Skills chargés depuis workspace/skills/
5. Long-term memory + daily notes
6. Top-K ranked memories (pertinentes)
7. Session history (max 50 messages)
8. Message utilisateur courant

### 4. Tools (11 outils obligatoires)

| Outil        | Description             | Sécurité                                                                    |
| ------------ | ----------------------- | --------------------------------------------------------------------------- |
| filesystem   | Read/write/list files   | Validation path + canonicalize, NO symlinks                                 |
| exec         | Execute shell commands  | Array args only (no shell), blacklist: rm, sudo, dd, mkfs, shutdown, reboot |
| web          | Fetch URLs              | Timeout 30s, max 1MB response                                               |
| message      | Send via chat hub       | -                                                                           |
| spawn        | Background tasks (stub) | -                                                                           |
| cron         | Schedule jobs           | Min 2min interval for recurring                                             |
| write_memory | Persist memory          | Écrit dans workspace/memory/                                                |
| create_skill | Create skill package    | Validation nom + path                                                       |
| list_skills  | List available skills   | -                                                                           |
| read_skill   | Read skill content      | -                                                                           |
| delete_skill | Remove skill            | Confirmation + backup ?                                                     |

### 5. Memory System

- Short-term : VecDeque in-memory (limit 100)
- Long-term : Fichier workspace/memory/MEMORY.md
- Daily notes : workspace/memory/YYYY-MM-DD.md
- Méthodes : add_short(), add_long(), recent(n), query_keyword()
- Ranker simple : Score par overlap de mots-clés + récence
- Ranker LLM (optionnel v2) : Appel LLM pour scorer la pertinence

### 6. Session Manager

- Clé : "{channel}:{chat_id}"
- Stockage : JSON files dans workspace/sessions/
- Max 50 messages, FIFO quand limite atteinte
- Thread-safe avec RwLock ou Mutex

### 7. Cron Scheduler

- In-memory uniquement (pas de persistance)
- Types : one-time (FireAt) et recurring (Interval)
- Min interval : 2 minutes pour recurring
- Callback qui pousse dans hub.inbound

### 8. Telegram Channel

- Long-polling avec 30s timeout
- Filtre allow_from (whitelist user IDs)
- Parse messages texte uniquement (v1)
- Envoi réponse via HTTP POST

### 9. Configuration

- Fichier : ~/.rustbot/config.json
- Structure : agents, providers, channels
- Env var override : OPENAI_API_KEY, TELEGRAM_BOT_TOKEN, etc.
- Commande onboard : crée config + workspace avec fichiers bootstrap

### 10. Workspace Structure (créé par onboard)

```
    ~/.rustbot/
    ├── config.json
    └── workspace/
    ├── SOUL.md # Personnalité
    ├── AGENTS.md # Instructions
    ├── USER.md # Profil utilisateur
    ├── TOOLS.md # Doc outils
    ├── HEARTBEAT.md # Tâches périodiques
    ├── memory/
    │ ├── MEMORY.md
    │ └── YYYY-MM-DD.md
    ├── sessions/ # JSON files
    └── skills/ # Skill packages
```

## Sécurité (CRITIQUE)

1. Filesystem :

- Toujours utiliser std::fs::canonicalize() pour résoudre les paths
- Vérifier que le path résolu commence par workspace/
- Interdire les symlinks (ou résoudre et revérifier)
- Pas de .. ou ~ dans les paths

2. Exec :

- UNIQUEMENT array de args : ["ls", "-la"] PAS ["ls -la"]
- Pas de shell intermédiaire (sh, bash, zsh)
- Blacklist strict : rm, sudo, dd, mkfs, fdisk, shutdown, reboot, etc.
- Timeout 30s par défaut, max 5min

3. Configuration sensible :

- API keys en env vars ou fichier 0600 permissions
- Jamais logguer les tokens

4. Telegram :

- Whitelist obligatoire (allow_from)
- Ignorer messages non autorisés

## CLI Commands

```cli
rustbot version                         # Version
rustbot onboard                         # Init config + workspace
rustbot agent -m "..."                  # Single query
rustbot agent -M model -m "..."         # With specific model
rustbot gateway                         # Start daemon mode
rustbot memory read today|long          # Read memory
rustbot memory append today|long -c "..." # Append
rustbot memory write long -c "..."      # Overwrite
rustbot memory recent --days N          # Recent N days
rustbot memory rank -q "query"          # Search relevant
```

## Critères de Qualité

- [ ] Binaire release < 15MB (avec cargo build --release + strip)
- [ ] RAM usage < 30MB idle
- [ ] Démarrage < 100ms
- [ ] Zero unsafe code (ou documenté si nécessaire)
- [ ] Tests unitaires pour tools et core logic
- [ ] Gestion d'erreurs gracieuse (pas de panic sur input user)
- [ ] Logging structuré avec tracing
- [ ] Docker image Alpine < 50MB

## Contraintes de Développement

1. Minimalisme : Moins de 10 dépendances externes si possible
2. Standard library first : Utiliser std avant crates externes
3. Async/await : Toutes les I/O doivent être async
4. Error handling : Propager avec ?, jamais unwrap() sur input extérieur
5. Documentation : Doc comments sur tous les traits et structs publiques

## Phases de Développement suggérées

```
Phase 1 - Core : Config, Chat Hub, Provider, Agent Loop basique
Phase 2 - Tools : Filesystem, Exec, Web, Memory
Phase 3 - Channels : Telegram integration
Phase 4 - Advanced : Cron, Skills, Session persistence, Ranking
Phase 5 - Polish : CLI complète, Docker, Tests, Docs
```
