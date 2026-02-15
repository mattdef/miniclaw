---
stepsCompleted:
  - step-01-validate-prerequisites
  - step-02-design-epics
  - step-03-create-stories
  - step-04-final-validation
inputDocuments:
  - prd.md
  - architecture.md
  - product-brief-miniclaw-2026-02-14.md
epicCount: 11
storyCount: 42
frCoverage: 100%
status: complete
---

# miniclaw - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for miniclaw, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

**FR1** : L'utilisateur peut afficher la version de miniclaw via la commande `version`

**FR2** : L'utilisateur peut initialiser la configuration et le workspace via la commande `onboard`

**FR3** : L'utilisateur peut envoyer une requête unique à l'agent via `agent -m "message"`

**FR4** : L'utilisateur peut spécifier un modèle LLM particulier via `agent -M model -m "message"`

**FR5** : L'utilisateur peut démarrer le mode daemon via la commande `gateway`

**FR6** : L'utilisateur peut lire la mémoire (today ou long) via `memory read`

**FR7** : L'utilisateur peut ajouter du contenu à la mémoire via `memory append`

**FR8** : L'utilisateur peut écraser la mémoire long terme via `memory write`

**FR9** : L'utilisateur peut consulter les mémoires des N derniers jours via `memory recent --days N`

**FR10** : L'utilisateur peut rechercher des mémoires pertinentes via `memory rank -q "query"`

**FR11** : L'utilisateur peut activer le mode verbeux pour le débogage via `--verbose`

**FR12** : L'agent peut recevoir des messages via le Chat Hub (channels tokio mpsc)

**FR13** : L'agent peut traiter jusqu'à 200 itérations d'outils par message

**FR14** : L'agent peut assembler un contexte complet (System + Bootstrap + Memory + Skills + History + Message)

**FR15** : L'agent peut appeler des outils et recevoir leurs résultats

**FR16** : L'agent peut répondre via le canal de communication approprié

**FR17** : L'agent peut maintenir une session de conversation persistante (max 50 messages FIFO)

**FR18** : Le système peut stocker la mémoire à court terme (VecDeque, limit 100 entrées)

**FR19** : Le système peut persister la mémoire long terme dans un fichier MEMORY.md

**FR20** : Le système peut créer des notes quotidiennes automatiques (YYYY-MM-DD.md)

**FR21** : Le système peut récupérer les mémoires les plus récentes

**FR22** : Le système peut classer les mémoires par pertinence (ranker simple par mots-clés)

**FR23** : Le système peut écrire de nouvelles mémoires via l'outil write_memory

**FR24** : Le système peut lire, écrire et lister des fichiers via l'outil filesystem

**FR25** : Le système peut exécuter des commandes shell via l'outil exec (avec restrictions de sécurité)

**FR26** : Le système peut récupérer du contenu web via l'outil web

**FR27** : Le système peut envoyer des messages via le Chat Hub via l'outil message

**FR28** : Le système peut lancer des tâches en arrière-plan via l'outil spawn

**FR29** : Le système peut planifier des tâches ponctuelles (FireAt) via l'outil cron

**FR30** : Le système peut planifier des tâches récurrentes (Interval, min 2min) via l'outil cron

**FR31** : Le système peut créer des packages de skills via l'outil create_skill

**FR32** : Le système peut lister les skills disponibles via l'outil list_skills

**FR33** : Le système peut lire le contenu d'un skill via l'outil read_skill

**FR34** : Le système peut supprimer un skill via l'outil delete_skill

**FR35** : Le système peut recevoir des messages via Telegram (long-polling, timeout 30s)

**FR36** : Le système peut filtrer les messages Telegram par whitelist (allow_from)

**FR37** : Le système peut envoyer des réponses via Telegram

**FR38** : Le système peut traiter uniquement les messages texte via Telegram (MVP)

**FR39** : Le système peut charger la configuration depuis un fichier JSON (~/.miniclaw/config.json)

**FR40** : Le système peut surcharger la configuration via des variables d'environnement

**FR41** : Le système peut accepter des overrides via des flags CLI

**FR42** : Le système peut créer automatiquement la structure de workspace (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md)

**FR43** : Le système peut charger les skills depuis le dossier workspace/skills/

**FR44** : Le système peut persister les sessions dans workspace/sessions/

**FR45** : Le système peut logger des messages aux niveaux ERROR, WARN, INFO, DEBUG

**FR46** : Le système peut sortir les logs sur stderr et les résultats sur stdout

**FR47** : Le système peut afficher des métriques de performance (utilisation RAM, temps de réponse)

### NonFunctional Requirements

**NFR-P1** : Le binaire compilé ne doit pas dépasser 15MB (mesuré avec `strip`)

**NFR-P2** : La consommation RAM au repos ne doit pas dépasser 30MB sur Raspberry Pi 3

**NFR-P3** : Le temps de démarrage (cold start) doit être inférieur à 100ms

**NFR-P4** : Le temps de réponse aux messages Telegram doit être inférieur à 2 secondes (95e percentile)

**NFR-P5** : Le système doit supporter jusqu'à 100 messages en attente dans le buffer du Chat Hub sans perte

**NFR-S1** : Les clés API et tokens doivent être stockés uniquement dans des variables d'environnement ou fichier avec permissions 0600

**NFR-S2** : Aucun secret ne doit apparaître dans les logs (même en mode verbose)

**NFR-S3** : Les chemins de fichiers doivent être validés et résolus via `canonicalize()` pour prévenir les attaques path traversal

**NFR-S4** : L'outil exec doit refuser d'exécuter les commandes blacklisted (rm, sudo, dd, mkfs, shutdown, reboot, etc.)

**NFR-S5** : Seuls les utilisateurs présents dans la whitelist Telegram peuvent interagir avec l'agent

**NFR-S6** : Les communications avec les APIs LLM doivent utiliser HTTPS/TLS 1.2 minimum

**NFR-R1** : Le système doit redémarrer automatiquement en cas de crash (avec systemd ou docker --restart)

**NFR-R2** : Les erreurs doivent être loggées avec niveau ERROR et message explicite

**NFR-R3** : Le système doit jamais panic sur une entrée utilisateur invalide

**NFR-R4** : Les sessions doivent être persistées automatiquement toutes les 30 secondes

**NFR-R5** : Le système doit supporter une interruption gracieuse (SIGTERM) avec flush des données

**NFR-C1** : Le système doit fonctionner sur Linux ARM64 (Raspberry Pi 3/4)

**NFR-C2** : Le système doit fonctionner sur Linux AMD64 (VPS, mini-PC x86)

**NFR-C3** : Le binaire ne doit avoir aucune dépendance runtime externe (libc standard uniquement)

**NFR-C4** : La configuration doit être compatible avec Docker et Docker Compose

**NFR-C5** : Le système doit fonctionner sur Windows x86-64

### Additional Requirements

**Technical Requirements from Architecture:**

- **Starter Template**: Architecture specifies using `cargo init` with modular Rust architecture - this will be Epic 1 Story 1
- **Technology Stack**: Rust 1.85+ Edition 2024, tokio, reqwest, serde, clap, chrono, regex, anyhow/thiserror, tracing, teloxide
- **Data Persistence**: JSON files for sessions, markdown files for memory, no database
- **Serialization**: serde_json for all JSON serialization
- **Concurrency**: Arc<RwLock<HashMap>> for session management, tokio mpsc channels for Chat Hub
- **Session Rotation**: TTL 30 days with sliding window for inactive sessions
- **Auto-persistence**: Sessions saved every 30 seconds via background task
- **Zero unsafe code policy**: All code must be safe Rust
- **Path validation**: All filesystem paths must be canonicalized
- **Security sandboxing**: Exec tool must have blacklist restrictions

**Project Structure Requirements:**

- Single crate with modules: config/, chat/, agent/, tools/, memory/, channels/, session/, cron/, providers/
- Workspace structure: ~/.miniclaw/ with config.json, workspace/ (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md, memory/, sessions/, skills/)
- File permissions: 0600 on config.json

**Implementation Patterns:**

- Rust RFC 430 naming conventions (snake_case for modules/functions, PascalCase for types)
- Trait-based extensibility (LLMProvider, Tool, Channel)
- Builder pattern for ContextBuilder
- Registry pattern for tools
- Async/await with tokio for all I/O
- Co-located tests in `#[cfg(test)]` modules
- Structured logging with tracing (never log secrets)

### FR Coverage Map

| FR | Epic | Description |
|----|------|-------------|
| FR1 | Epic 1 | Afficher la version via `version` |
| FR2 | Epic 2 | Initialiser la configuration via `onboard` |
| FR3 | Epic 4 | Envoyer requête unique via `agent -m` |
| FR4 | Epic 4 | Spécifier modèle LLM via `agent -M` |
| FR5 | Epic 10 | Démarrer mode daemon via `gateway` |
| FR6 | Epic 8 | Lire la mémoire via `memory read` |
| FR7 | Epic 8 | Ajouter contenu mémoire via `memory append` |
| FR8 | Epic 8 | Écraser mémoire long terme via `memory write` |
| FR9 | Epic 8 | Consulter mémoires récentes via `memory recent` |
| FR10 | Epic 8 | Rechercher mémoires via `memory rank` |
| FR11 | Epic 1 | Activer mode verbeux via `--verbose` |
| FR12 | Epic 3 | Recevoir messages via Chat Hub (tokio mpsc) |
| FR13 | Epic 5 | Traiter jusqu'à 200 itérations d'outils |
| FR14 | Epic 5 | Assembler contexte complet (System + Bootstrap + Memory + Skills + History + Message) |
| FR15 | Epic 5 | Appeler des outils et recevoir résultats |
| FR16 | Epic 5 | Répondre via canal de communication approprié |
| FR17 | Epic 5, 9 | Maintenir session persistante (50 messages FIFO) |
| FR18 | Epic 8 | Stocker mémoire court terme (VecDeque 100) |
| FR19 | Epic 8 | Persister mémoire long terme dans MEMORY.md |
| FR20 | Epic 8 | Créer notes quotidiennes automatiques (YYYY-MM-DD.md) |
| FR21 | Epic 8 | Récupérer mémoires les plus récentes |
| FR22 | Epic 8 | Classer mémoires par pertinence (ranker mots-clés) |
| FR23 | Epic 7 | Écrire nouvelles mémoires via outil write_memory |
| FR24 | Epic 6 | Lire/écrire/lister fichiers via outil filesystem |
| FR25 | Epic 6 | Exécuter commandes shell via outil exec |
| FR26 | Epic 6 | Récupérer contenu web via outil web |
| FR27 | Epic 3 | Envoyer messages via Chat Hub via outil message |
| FR28 | Epic 6 | Lancer tâches arrière-plan via outil spawn |
| FR29 | Epic 7 | Planifier tâches ponctuelles (FireAt) via cron |
| FR30 | Epic 7 | Planifier tâches récurrentes (Interval min 2min) via cron |
| FR31 | Epic 7 | Créer packages de skills via create_skill |
| FR32 | Epic 7 | Lister skills disponibles via list_skills |
| FR33 | Epic 7 | Lire contenu skill via read_skill |
| FR34 | Epic 7 | Supprimer skill via delete_skill |
| FR35 | Epic 10 | Recevoir messages Telegram (long-polling 30s) |
| FR36 | Epic 10 | Filtrer messages Telegram par whitelist (allow_from) |
| FR37 | Epic 10 | Envoyer réponses via Telegram |
| FR38 | Epic 10 | Traiter uniquement messages texte Telegram (MVP) |
| FR39 | Epic 2 | Charger configuration depuis JSON (~/.miniclaw/config.json) |
| FR40 | Epic 2 | Surcharger configuration via variables d'environnement |
| FR41 | Epic 2 | Accepter overrides via flags CLI |
| FR42 | Epic 2 | Créer structure workspace (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md) |
| FR43 | Epic 2 | Charger skills depuis workspace/skills/ |
| FR44 | Epic 2, 9 | Persister sessions dans workspace/sessions/ |
| FR45 | Epic 11 | Logger messages niveaux ERROR, WARN, INFO, DEBUG |
| FR46 | Epic 11 | Sortir logs sur stderr et résultats sur stdout |
| FR47 | Epic 11 | Afficher métriques performance (RAM, temps réponse) |

## Epic List

### Epic 1: Project Foundation & Core Infrastructure
L'utilisateur peut installer miniclaw et utiliser les commandes CLI de base
**FRs covered:** FR1, FR11

### Epic 2: Configuration & Workspace Management
L'utilisateur peut configurer miniclaw et initialiser son workspace
**FRs covered:** FR2, FR39, FR40, FR41, FR42, FR43, FR44

### Epic 3: Chat Hub & Message Routing
Le système peut recevoir et router des messages via des channels
**FRs covered:** FR12, FR27

### Epic 4: LLM Provider Integration
Le système peut communiquer avec les providers LLM
**FRs covered:** FR3, FR4

### Epic 5: Agent Core - Loop & Context
L'agent peut exécuter son loop et assembler le contexte
**FRs covered:** FR13, FR14, FR15, FR16, FR17

### Epic 6: Tool System - Core Tools
Le système dispose des outils essentiels (filesystem, exec, web, spawn)
**FRs covered:** FR24, FR25, FR26, FR28

### Epic 7: Tool System - Advanced Tools
Le système dispose des outils avancés (cron, memory, skills)
**FRs covered:** FR29, FR30, FR23, FR31, FR32, FR33, FR34

### Epic 8: Memory System
Le système peut stocker et récupérer la mémoire court/long terme
**FRs covered:** FR6, FR7, FR8, FR9, FR10, FR18, FR19, FR20, FR21, FR22

### Epic 9: Session Management
Le système gère les sessions utilisateur avec persistance automatique
**FRs covered:** FR17, FR44

### Epic 10: Telegram Channel Integration
Le système peut communiquer via Telegram
**FRs covered:** FR5, FR35, FR36, FR37, FR38

### Epic 11: System Monitoring & Reliability
Le système est observable et résilient
**FRs covered:** FR45, FR46, FR47

<!-- Repeat for each epic in epics_list (N = 1, 2, 3...) -->

## Epic {{N}}: {{epic_title_N}}

{{epic_goal_N}}

<!-- Repeat for each story (M = 1, 2, 3...) within epic N -->

### Story {{N}}.{{M}}: {{story_title_N_M}}

As a {{user_type}},
I want {{capability}},
So that {{value_benefit}}.

**Acceptance Criteria:**

<!-- for each AC on this story -->

**Given** {{precondition}}
**When** {{action}}
**Then** {{expected_outcome}}
**And** {{additional_criteria}}

## Epic 1: Project Foundation & Core Infrastructure

L'utilisateur peut installer miniclaw et utiliser les commandes CLI de base

### Story 1.1: Version Command

As a user,
I want to check the miniclaw version from the command line,
So that I can verify my installation and check for updates.

**Acceptance Criteria:**

**Given** miniclaw is installed
**When** I run `miniclaw version`
**Then** the system displays the current semantic version (e.g., "miniclaw 0.1.0")
**And** the exit code is 0

**Given** miniclaw is installed
**When** I run `miniclaw --version`
**Then** the system displays the same version information
**And** the exit code is 0

**Given** miniclaw is installed
**When** I run `miniclaw -V`
**Then** the system displays the same version information
**And** the exit code is 0

### Story 1.2: CLI Framework Setup

As a user,
I want a robust CLI interface with proper argument parsing,
So that I can interact with miniclaw intuitively and get helpful error messages.

**Acceptance Criteria:**

**Given** miniclaw is installed
**When** I run `miniclaw` without any arguments
**Then** the system displays the help message with available commands
**And** the exit code is 0

**Given** miniclaw is installed
**When** I run `miniclaw invalid_command`
**Then** the system displays an error message "unknown command: invalid_command"
**And** suggests valid commands
**And** the exit code is 1

**Given** miniclaw is installed
**When** I run a command with invalid flags (e.g., `miniclaw version --invalid`)
**Then** the system displays an error about the unrecognized flag
**And** shows correct usage
**And** the exit code is 2

**Given** any CLI operation
**When** an error occurs
**Then** the error message is clear and actionable
**And** error output goes to stderr
**And** normal output goes to stdout

### Story 1.3: Verbose Logging Mode

As a developer or user debugging miniclaw,
I want to enable verbose logging mode,
So that I can see detailed operational information for troubleshooting.

**Acceptance Criteria:**

**Given** miniclaw is installed
**When** I run any command with `--verbose` flag
**Then** the system enables DEBUG level logging
**And** log messages include timestamps
**And** log messages include log level (ERROR, WARN, INFO, DEBUG)
**And** log messages include the source module

**Given** miniclaw is installed
**When** I run any command with `-v` short flag
**Then** the same verbose mode is activated as with `--verbose`

**Given** verbose mode is enabled
**When** the system executes operations
**Then** DEBUG logs show function entry/exit points
**And** DEBUG logs show key variable values (non-sensitive)
**And** INFO logs show major lifecycle events

**Given** verbose mode is enabled
**When** logging configuration data
**Then** API keys and tokens are NEVER logged
**And** sensitive paths are redacted

**Given** any command execution
**When** not in verbose mode (default)
**Then** only INFO, WARN, and ERROR levels are displayed
**And** DEBUG logs are suppressed

### Story 1.4: Help System

As a user,
I want comprehensive help documentation integrated in the CLI,
So that I can learn how to use miniclaw without external documentation.

**Acceptance Criteria:**

**Given** miniclaw is installed
**When** I run `miniclaw help`
**Then** the system displays the main help with all top-level commands
**And** shows command descriptions
**And** shows global flags

**Given** miniclaw is installed
**When** I run `miniclaw --help`
**Then** the system displays the same help content

**Given** miniclaw is installed  
**When** I run `miniclaw [command] help` (e.g., `miniclaw onboard help`)
**Then** the system displays help specific to that command
**And** shows command description
**And** shows available subcommands if any
**And** shows command-specific flags

**Given** miniclaw is installed
**When** I run `miniclaw [command] --help`
**Then** the system displays the same command-specific help

**Given** viewing help output
**Then** commands are grouped logically
**And** required arguments are clearly marked
**And** optional arguments show default values
**And** examples are provided for complex commands

<!-- End story repeat -->

## Epic 2: Configuration & Workspace Management

L'utilisateur peut configurer miniclaw et initialiser son workspace

### Story 2.1: Configuration File Management

As a user,
I want miniclaw to manage configuration through a flexible hierarchy,
So that I can customize behavior through files, environment variables, or command flags.

**Acceptance Criteria:**

**Given** a fresh miniclaw installation
**When** the system loads configuration
**Then** it applies default values first
**And** overrides with `~/.miniclaw/config.json` if it exists
**And** overrides with environment variables (e.g., `MINICLAW_API_KEY`)
**And** finally overrides with CLI flags

**Given** a config file at `~/.miniclaw/config.json`
**When** I define `api_key`, `model`, and `telegram_token` fields
**Then** the system loads and validates these values
**And** applies them to the application

**Given** environment variables are set
**When** I set `OPENROUTER_API_KEY=sk-xxx` or `TELEGRAM_BOT_TOKEN=xxx`
**Then** these values override config file settings
**And** secrets are prioritized from environment variables

**Given** I run a command with CLI flags
**When** I use `--model google/gemini-2.5-flash` or `--config /custom/path.json`
**Then** these values take highest precedence
**And** override both file and environment settings

**Given** the config file has invalid JSON
**When** miniclaw tries to load it
**Then** the system displays a clear error message
**And** suggests running `miniclaw onboard` to recreate it
**And** the exit code is 1

**Given** sensitive configuration data
**When** the config file is created
**Then** file permissions are set to 0600 (owner read/write only)
**And** no other users can read the file

### Story 2.2: Interactive Onboarding Command

As a new user,
I want an interactive onboarding command,
So that I can quickly set up miniclaw with guided configuration.

**Acceptance Criteria:**

**Given** miniclaw is installed for the first time
**When** I run `miniclaw onboard`
**Then** the system creates the `~/.miniclaw/` directory
**And** creates `~/.miniclaw/config.json` with default values
**And** displays "Workspace initialized successfully"

**Given** the onboarding wizard is running
**When** it prompts for API configuration
**Then** it asks for OpenRouter API key (with option to skip)
**And** explains where to get the key
**And** validates the key format (starts with `sk-or-`)

**Given** the onboarding wizard is running
**When** it prompts for Telegram configuration
**Then** it provides step-by-step instructions:
  - "Step 1: Message @BotFather on Telegram"
  - "Step 2: Type /newbot and follow instructions"
  - "Step 3: Copy the token here: [input]"
**And** validates the token format

**Given** the onboarding wizard is running
**When** it prompts for user identification
**Then** it explains how to find Telegram user ID
**And** asks for the whitelist user ID
**And** confirms the configuration

**Given** I run `miniclaw onboard --verbose`
**When** the onboarding executes
**Then** it displays detailed logging of each step
**And** shows which files are being created
**And** shows configuration values being set (with secrets masked)

**Given** the workspace already exists
**When** I run `miniclaw onboard` again
**Then** the system warns "Workspace already exists at ~/.miniclaw/"
**And** asks "Do you want to reconfigure? (y/N)"
**And** preserves existing data if I choose 'N'

**Given** onboarding completes successfully
**When** the final configuration is saved
**Then** the system displays a summary of configured values
**And** shows next steps: "Run 'miniclaw gateway' to start"

### Story 2.3: Workspace Structure Creation

As a user,
I want miniclaw to create a complete workspace structure,
So that I can customize my agent's personality and capabilities.

**Acceptance Criteria:**

**Given** onboarding is running
**When** the system creates the workspace
**Then** it creates `~/.miniclaw/workspace/` directory

**Given** workspace directory creation
**When** initializing the workspace
**Then** it creates `workspace/SOUL.md` with default agent personality template
**And** the file contains sections for agent name, personality traits, and communication style

**Given** workspace initialization
**When** creating agent instructions
**Then** it creates `workspace/AGENTS.md` with default agent behavior guidelines
**And** includes sections for available tools and usage patterns

**Given** workspace initialization
**When** setting up user profile
**Then** it creates `workspace/USER.md` with placeholder for user information
**And** includes sections for preferences and context

**Given** workspace initialization
**When** creating tool documentation
**Then** it creates `workspace/TOOLS.md` documenting all available tools
**And** includes usage examples for each tool
**And** describes parameter formats

**Given** workspace initialization
**When** setting up periodic tasks
**Then** it creates `workspace/HEARTBEAT.md` for scheduled task definitions
**And** includes example cron jobs
**And** explains the heartbeat system

**Given** all workspace files are created
**When** a file is missing or corrupted
**Then** the system can recreate it individually
**And** preserves other existing files

**Given** workspace files exist
**When** the agent assembles context
**Then** it loads SOUL.md as system personality
**And** loads AGENTS.md as behavioral guidelines
**And** loads USER.md for user context
**And** loads TOOLS.md for tool documentation
**And** loads HEARTBEAT.md for scheduled tasks

### Story 2.4: Skills Directory Setup

As a user,
I want a dedicated directory for custom skills,
So that I can extend miniclaw's capabilities with reusable skill packages.

**Acceptance Criteria:**

**Given** workspace initialization
**When** the system creates the workspace structure
**Then** it creates `~/.miniclaw/workspace/skills/` directory

**Given** the skills directory exists
**When** I create a new skill folder (e.g., `skills/weather/`)
**Then** miniclaw recognizes it as a valid skill package

**Given** a skill directory exists
**When** I add a `SKILL.md` file inside it
**Then** the system parses the skill definition
**And** makes it available to the agent

**Given** a skill package with SKILL.md
**When** the agent loads skills
**Then** it reads the skill name, description, and parameters
**And** validates the skill format

**Given** multiple skills exist in the skills directory
**When** the agent assembles context
**Then** it loads all valid skills
**And** includes them in the available tools context

**Given** a skill has invalid format
**When** the system tries to load it
**Then** it logs a warning about the invalid skill
**And** skips that skill but continues loading others

**Given** I want to disable a skill
**When** I rename the directory with a dot prefix (e.g., `.weather/`)
**Then** the system ignores that skill
**And** does not load it into the agent context

**Given** the `list_skills` tool is called
**When** the agent executes it
**Then** it returns a list of all available skills
**And** shows skill names and descriptions
**And** indicates which skills are active

### Story 2.5: Sessions Directory Setup

As a user,
I want persistent session storage,
So that my conversations are preserved across restarts.

**Acceptance Criteria:**

**Given** workspace initialization
**When** the system creates the workspace structure
**Then** it creates `~/.miniclaw/workspace/sessions/` directory

**Given** a conversation starts
**When** the session manager initializes
**Then** it creates a session file at `sessions/{channel}_{chat_id}.json`
**And** uses format like `telegram_123456789.json`

**Given** a session file format
**When** sessions are persisted
**Then** JSON structure includes:
  - `session_id`: unique identifier
  - `channel`: communication channel (e.g., "telegram")
  - `chat_id`: user identifier
  - `created_at`: ISO 8601 timestamp
  - `last_accessed`: ISO 8601 timestamp
  - `messages`: array of message objects (max 50, FIFO)

**Given** a session exists
**When** a new message is added
**Then** the message includes `role` (user/assistant)
**And** includes `content` (message text)
**And** includes `timestamp` (ISO 8601)
**And** includes optional `tool_calls` array

**Given** session has 50 messages
**When** a 51st message arrives
**Then** the oldest message is removed (FIFO)
**And** the new message is added
**And** the session maintains max 50 messages

**Given** session files exist
**When** the system starts
**Then** it loads existing sessions from the directory
**And** makes them available for resumed conversations

**Given** session persistence
**When** files are saved
**Then** they use snake_case field names in JSON
**And** timestamps are ISO 8601 UTC format
**And** files have 0600 permissions

**Given** a session file is corrupted
**When** the system tries to load it
**Then** it logs an error
**And** creates a new empty session
**And** preserves the corrupted file with `.corrupted` suffix

## Epic 3: Chat Hub & Message Routing

Le système peut recevoir et router des messages via des channels

### Story 3.1: Chat Hub Core Infrastructure

As a developer,
I want a central message routing system,
So that messages can flow between channels and the agent efficiently.

**Acceptance Criteria:**

**Given** the Chat Hub is initialized
**When** the system starts
**Then** it creates inbound channel (mpsc) with buffer size 100
**And** creates outbound channel (mpsc) with buffer size 100
**And** both channels use tokio::sync::mpsc

**Given** an inbound message arrives
**When** it is sent to the Chat Hub
**Then** the message is wrapped in InboundMessage struct
**And** includes channel identifier (e.g., "telegram")
**And** includes chat_id (user identifier)
**And** includes content (message text)
**And** includes metadata HashMap

**Given** the agent needs to send a reply
**When** it creates an OutboundMessage
**Then** the message includes channel identifier
**And** includes chat_id (destination)
**And** includes content (reply text)
**And** optionally includes reply_to (message_id)

**Given** multiple channels are connected
**When** the Hub routes a message
**Then** it dispatches to the correct channel based on channel identifier
**And** maintains isolation between different channels

**Given** the inbound buffer reaches capacity (100 messages)
**When** a new message arrives
**Then** the oldest message is dropped (FIFO)
**And** a warning is logged about buffer overflow

**Given** the Chat Hub is running
**When** the system receives SIGTERM
**Then** it drains both channels gracefully
**And** processes remaining messages before shutdown

### Story 3.2: Inbound Message Processing

As an agent,
I want to receive and process incoming messages,
So that I can respond to user requests.

**Acceptance Criteria:**

**Given** a message arrives from Telegram
**When** it is received by the Chat Hub
**Then** it is converted to InboundMessage format
**And** timestamp is recorded (ISO 8601 UTC)

**Given** an inbound message
**When** the agent loop processes it
**Then** the message content is extracted
**And** the chat_id is identified for routing replies
**And** the channel is identified for protocol-specific handling

**Given** a message contains only whitespace
**When** it is processed
**Then** it is ignored (not sent to agent)
**And** no reply is generated

**Given** a message is too long (>4000 characters for Telegram)
**When** it is processed
**Then** it is truncated or split appropriately
**And** a warning is logged

**Given** concurrent messages from multiple users
**When** they arrive simultaneously
**Then** each is processed independently
**And** no messages are lost
**And** replies are routed to correct users

### Story 3.3: Outbound Message Delivery

As an agent,
I want to send replies back to users,
So that I can communicate responses and results.

**Acceptance Criteria:**

**Given** the agent generates a response
**When** it sends the message to Chat Hub
**Then** the Hub creates an OutboundMessage
**And** routes it to the correct channel

**Given** an outbound message
**When** it is delivered to the channel adapter
**Then** the channel-specific protocol is used
**And** for Telegram: uses teloxide send_message

**Given** a message delivery fails
**When** the channel returns an error
**Then** the error is logged with level ERROR
**And** the message may be retried based on channel policy
**And** the agent is notified of delivery failure

**Given** the outbound buffer is full
**When** a new message needs to be sent
**Then** the system waits briefly for space
**And** if still full, drops oldest message
**And** logs a warning

**Given** a reply references a specific message
**When** it is sent via Telegram
**Then** it uses reply_to_message_id for threading
**And** the user sees it as a reply in Telegram

### Story 3.4: Message Tool Integration

As an agent,
I want a tool to send messages programmatically,
So that I can notify users proactively.

**Acceptance Criteria:**

**Given** the agent is executing tools
**When** the `message` tool is called
**Then** it accepts parameters: chat_id, content, optional channel

**Given** the message tool is invoked
**When** it receives valid parameters
**Then** it creates an OutboundMessage
**And** sends it to the Chat Hub outbound channel
**And** returns success confirmation

**Given** the message tool is invoked without channel
**When** the agent context has a default channel
**Then** it uses the channel from the current conversation
**And** routes to the same channel as the incoming message

**Given** the message tool targets an invalid chat_id
**When** it attempts to send
**Then** it returns an error describing the issue
**And** does not crash the agent loop

**Given** the agent calls message tool
**When** the message is successfully queued
**Then** the tool returns immediately (non-blocking)
**And** actual delivery happens asynchronously


## Epic 4: LLM Provider Integration

Le système peut communiquer avec les providers LLM

### Story 4.1: LLM Provider Trait and Architecture

As a developer,
I want a provider-agnostic interface for LLMs,
So that miniclaw can work with multiple LLM services.

**Acceptance Criteria:**

**Given** the provider module is implemented
**When** defining the LLMProvider trait
**Then** it requires `chat()` method for completions
**And** it requires `default_model()` method
**And** it is Send + Sync for thread safety

**Given** the chat method signature
**When** implementing a provider
**Then** it accepts: messages (Vec<Message>), tools (Vec<ToolDefinition>), model (&str)
**And** returns: Result<LLMResponse>

**Given** the Message type
**When** creating conversation messages
**Then** it has role enum: System, User, Assistant
**And** has content field (String)
**And** optionally has tool_calls field

**Given** the LLMResponse type
**When** parsing provider responses
**Then** it contains content (String, assistant message)
**And** optionally contains tool_calls (Vec<ToolCall>)
**And** contains usage statistics (prompt_tokens, completion_tokens)

**Given** the ToolCall type
**When** the LLM requests tool execution
**Then** it contains id (unique identifier)
**And** contains name (tool name)
**And** contains arguments (JSON object as string)

### Story 4.2: OpenAI-Compatible Provider

As a user,
I want to connect to OpenAI-compatible APIs (OpenRouter),
So that I can use various LLM models.

**Acceptance Criteria:**

**Given** OpenRouter API key is configured
**When** the agent needs to call an LLM
**Then** it uses reqwest to make HTTPS POST to OpenRouter API
**And** includes Authorization header with Bearer token
**And** sets Content-Type: application/json

**Given** an API request
**When** constructing the request body
**Then** it includes model field (e.g., "google/gemini-2.5-flash")
**And** includes messages array in OpenAI format
**And** includes tools array with function definitions
**And** includes tool_choice: "auto"

**Given** the API returns a successful response
**When** parsing the JSON
**Then** it extracts the assistant message content
**And** extracts any tool_calls from the response
**And** extracts token usage statistics

**Given** the API returns an error
**When** handling the response
**Then** it distinguishes HTTP errors (4xx, 5xx)
**And** distinguishes API errors (invalid key, rate limit)
**And** provides clear error messages to the agent

**Given** API rate limiting occurs
**When** receiving 429 status
**Then** it implements exponential backoff retry
**And** retries up to 3 times with delays
**And** eventually returns error if all retries fail

**Given** network connectivity issues
**When** the request times out
**Then** timeout is set to 30 seconds
**And** returns timeout error after that period
**And** does not block indefinitely

### Story 4.3: Ollama Local Provider

As a privacy-conscious user,
I want to use local LLMs via Ollama,
So that my data never leaves my machine.

**Acceptance Criteria:**

**Given** Ollama is running locally on port 11434
**When** miniclaw is configured with Ollama provider
**Then** it makes HTTP requests to `http://localhost:11434/api/chat`
**And** uses the Ollama chat API format

**Given** Ollama provider is selected
**When** calling the chat endpoint
**Then** it sends model name (e.g., "llama3.2", "mistral")
**And** sends messages in Ollama format
**And** optionally sends tools if model supports it

**Given** Ollama returns a streaming response
**When** processing the chunks
**Then** it accumulates the full response
**And** handles the stream properly
**And** returns complete message when done

**Given** Ollama is not running
**When** attempting to connect
**Then** it detects connection refused error
**And** suggests "Is Ollama running? Start it with: ollama serve"
**And** provides clear troubleshooting steps

**Given** the requested model is not available
**When** Ollama returns 404
**Then** it suggests running `ollama pull [model_name]`
**And** lists available models via `ollama list`

**Given** a local LLM is used
**When** tracking token usage
**Then** it estimates tokens if provider doesn't return them
**And** logs that usage is approximate

### Story 4.4: Agent One-Shot Command

As a user,
I want to send single messages to the agent via CLI,
So that I can interact without running the full gateway.

**Acceptance Criteria:**

**Given** miniclaw is installed and configured
**When** I run `miniclaw agent -m "Hello"`
**Then** it loads configuration from `~/.miniclaw/config.json`
**And** initializes a temporary session
**And** sends the message to the agent

**Given** the agent command executes
**When** the LLM responds
**Then** the response is printed to stdout
**And** the exit code is 0 on success
**And** the program terminates after response

**Given** I want to use a specific model
**When** I run `miniclaw agent -M "anthropic/claude-3.5-sonnet" -m "Hello"`
**Then** it overrides the default model from config
**And** uses the specified model for this request only

**Given** the agent encounters an error
**When** executing the command
**Then** error is printed to stderr
**And** exit code is 1
**And** verbose mode shows stack trace if enabled

**Given** the message is long or complex
**When** the agent processes it
**Then** it can invoke tools as needed
**And** returns final response after tool executions
**And** shows progress if verbose mode is on


## Epic 5: Agent Core - Loop & Context

L'agent peut exécuter son loop et assembler le contexte

### Story 5.1: Agent Loop Implementation

As an agent,
I want a main execution loop,
So that I can process messages and coordinate tool calls.

**Acceptance Criteria:**

**Given** a message arrives from Chat Hub
**When** the Agent Loop starts processing
**Then** it initializes iteration counter to 0
**And** begins the Receive→Context→LLM→Tools→Reply cycle

**Given** the agent is in the loop
**When** it reaches the Context phase
**Then** it calls ContextBuilder to assemble context
**And** includes System + Bootstrap + Memory + Skills + History + Current Message

**Given** context is assembled
**When** calling the LLM
**Then** it sends context messages to LLMProvider
**And** includes available tool definitions
**And** waits for LLM response

**Given** the LLM responds with text only
**When** no tool calls are present
**Then** the loop terminates
**And** the text is sent as reply to user
**And** the final iteration count is logged

**Given** the LLM responds with tool calls
**When** tools need to be executed
**Then** iteration counter increments
**And** each tool is executed (potentially in parallel)
**And** results are collected

**Given** tool execution completes
**When** results are available
**Then** they are formatted as tool result messages
**And** added to conversation history
**And** the loop continues to next iteration

**Given** iteration counter reaches 200
**When** the loop would continue
**Then** it terminates with error "Max iterations reached"
**And** returns partial results to user
**And** logs warning about potential infinite loop

**Given** a tool execution fails
**When** the error is captured
**Then** it is formatted as error result
**And** included in context for LLM
**And** the loop continues (does not crash)

### Story 5.2: Context Builder

As an agent,
I want to assemble complete conversation context,
So that the LLM has all necessary information to respond.

**Acceptance Criteria:**

**Given** building context for a conversation
**When** the ContextBuilder assembles components
**Then** it includes layers in this order:
  1. System prompt (from SOUL.md + AGENTS.md)
  2. Bootstrap context
  3. Long-term memory (from MEMORY.md)
  4. Available skills (from skills/)
  5. Tool documentation (from TOOLS.md)
  6. Conversation history (from session, max 50 messages)
  7. Current user message

**Given** the System layer
**When** loading from SOUL.md and AGENTS.md
**Then** SOUL.md provides personality and name
**And** AGENTS.md provides behavior guidelines
**And** combined into initial system message

**Given** the Memory layer
**When** relevant memories are found
**Then** they are formatted as context
**And** ranked by relevance using ranker
**And** limited to avoid context overflow

**Given** the Skills layer
**When** skills are loaded from workspace
**Then** each SKILL.md is parsed
**And** formatted as available capabilities
**And** included in system context

**Given** the Tools layer
**When** TOOLS.md is loaded
**Then** it documents available tools
**And** explains parameter formats
**And** provides usage examples

**Given** conversation history
**When** loading from session
**Then** up to 50 most recent messages are included
**And** messages are formatted with role and content
**And** tool calls and results are preserved

**Given** context assembly
**When** total size approaches token limit
**Then** older conversation messages are truncated first
**And** system prompt is never truncated
**And** current message is always included

### Story 5.3: Session Management in Agent Loop

As a user,
I want my conversation history preserved,
So that the agent remembers context across messages.

**Acceptance Criteria:**

**Given** a conversation starts
**When** the first message arrives
**Then** a new session is created
**And** session_id is generated: "{channel}_{chat_id}"
**And** created_at timestamp is recorded

**Given** a session exists
**When** a message is processed
**Then** user message is added to session
**And** assistant response is added to session
**And** last_accessed timestamp is updated

**Given** session reaches 50 messages
**When** a new message arrives
**Then** the oldest message is removed
**And** the new message is appended
**And** session maintains FIFO order

**Given** a conversation has tool interactions
**When** tools are called
**Then** tool_calls are stored in assistant message
**And** tool results are stored as separate messages
**And** the interaction flow is preserved

**Given** session persistence is configured
**When** 30 seconds elapse
**Then** sessions are automatically saved to disk
**And** saved to `~/.miniclaw/workspace/sessions/`
**And** save happens in background (non-blocking)

**Given** the system restarts
**When** it loads previous sessions
**Then** it reads all session files from disk
**And** makes them available for resumed conversations
**And** maintains conversation continuity


## Epic 6: Tool System - Core Tools

Le système dispose des outils essentiels (filesystem, exec, web, spawn)

### Story 6.1: Tool Registry and Trait

As a developer,
I want a flexible tool system,
So that tools can be registered and executed dynamically.

**Acceptance Criteria:**

**Given** the Tool trait is defined
**When** implementing a new tool
**Then** it requires `name()` returning &str
**And** requires `description()` returning &str
**And** requires `parameters()` returning JSON Schema
**And** requires `execute()` async method

**Given** the ToolRegistry
**When** tools are registered
**Then** they are stored in a HashMap by name
**And** can be retrieved by name
**And** names must be unique

**Given** a tool registration conflict
**When** registering a tool with duplicate name
**Then** it returns error "Tool already exists"
**And** suggests using different name

**Given** the registry at runtime
**When** listing available tools
**Then** it returns all registered tools
**And** includes name, description, and parameter schema for each

**Given** tool definitions for LLM
**When** formatting for API calls
**Then** each tool is converted to OpenAI function format
**And** includes name, description, and parameters schema

### Story 6.2: Filesystem Tool

As an agent,
I want to read, write, and list files,
So that I can interact with the filesystem.

**Acceptance Criteria:**

**Given** the filesystem tool is called
**When** operation is "read"
**Then** it accepts parameter: path
**And** validates path with canonicalize()
**And** prevents path traversal attacks

**Given** a read operation
**When** the file exists
**Then** it returns file contents as string
**And** handles text files (UTF-8)

**Given** a read operation on non-existent file
**When** the file doesn't exist
**Then** it returns error "File not found: {path}"
**And** exit code indicates failure

**Given** the filesystem tool is called
**When** operation is "write"
**Then** it accepts parameters: path, content
**And** creates parent directories if needed
**And** writes content to file
**And** returns success confirmation

**Given** a write operation would overwrite
**When** file already exists
**Then** it warns "File exists, overwriting"
**And** proceeds with write
**And** logs the action

**Given** the filesystem tool is called
**When** operation is "list"
**Then** it accepts parameter: path (directory)
**And** returns list of files and directories
**And** includes names and types (file/dir)

**Given** a list operation on non-directory
**When** path is a file
**Then** it returns error "Path is not a directory"

**Given** path validation
**When** any filesystem operation
**Then** path is resolved with canonicalize()
**And** must be within allowed base directory
**And** prevents access to sensitive paths (/etc, /root, etc.)

### Story 6.3: Exec Tool

As an agent,
I want to execute shell commands,
So that I can run system utilities and scripts.

**Acceptance Criteria:**

**Given** the exec tool is called
**When** executing a command
**Then** it accepts parameters: command, args (array), optional cwd
**And** args must be provided as array (prevents shell injection)

**Given** a command execution request
**When** checking against blacklist
**Then** it rejects: rm, sudo, dd, mkfs, shutdown, reboot, passwd, visudo
**And** returns error "Command not allowed: {command}"

**Given** a whitelisted command
**When** executing it
**Then** it runs with timeout (30 seconds default)
**And** captures stdout and stderr
**And** returns combined output

**Given** command execution completes
**When** capturing output
**Then** stdout is captured
**And** stderr is captured
**And** exit code is captured
**And** all are returned to agent

**Given** a command times out
**When** execution exceeds timeout
**Then** the process is killed
**And** error "Command timed out after {timeout}s" is returned

**Given** a command fails
**When** exit code is non-zero
**Then** output is still returned
**And** exit code is included
**And** agent is informed of failure

**Given** optional cwd parameter
**When** provided
**Then** command executes in specified directory
**And** path is validated (same as filesystem tool)

### Story 6.4: Web Tool

As an agent,
I want to fetch web content,
So that I can access information from the internet.

**Acceptance Criteria:**

**Given** the web tool is called
**When** fetching a URL
**Then** it accepts parameter: url
**And** validates URL format
**And** requires http:// or https:// protocol

**Given** a valid URL
**When** making the request
**Then** it uses reqwest for HTTP GET
**And** follows redirects (up to 5)
**And** timeout is 30 seconds

**Given** a successful fetch
**When** response is received
**Then** it returns response body as string
**And** extracts text content
**And** returns HTTP status code

**Given** the response is HTML
**When** processing content
**Then** it extracts text content (strips tags)
**And** preserves readable text structure
**And** limits response size (max 100KB)

**Given** the response is JSON
**When** content type is application/json
**Then** it returns raw JSON string
**And** agent can parse if needed

**Given** a fetch fails
**When** network error occurs
**Then** it returns error message
**And** includes underlying error details
**And** suggests checking URL or connectivity

**Given** HTTP error status
**When** receiving 4xx or 5xx
**Then** it returns error with status code
**And** includes response body if available
**And** explains the error type

### Story 6.5: Spawn Tool

As an agent,
I want to run background tasks,
So that I can execute long-running operations without blocking.

**Acceptance Criteria:**

**Given** the spawn tool is called
**When** spawning a process
**Then** it accepts parameters: command, args (array), optional cwd
**And** uses same validation as exec tool
**And** applies same command blacklist

**Given** a spawn request
**When** process starts
**Then** it returns immediately with process ID
**And** does not wait for completion
**And** process runs in background

**Given** a spawned process
**When** it completes
**Then** exit code is logged
**And** stdout/stderr may be logged (configurable)
**And** agent is NOT notified (fire-and-forget)

**Given** spawn limitations
**When** tracking spawned processes
**Then** agent can view active processes via system tools
**And** agent cannot directly communicate with spawned process

**Given** a spawn request fails
**When** command not found or invalid
**Then** error is returned immediately
**And** no process is created


## Epic 7: Tool System - Advanced Tools

Le système dispose des outils avancés (cron, memory, skills)

### Story 7.1: Cron Tool - Task Scheduling

As an agent,
I want to schedule tasks for later execution,
So that I can automate periodic or delayed actions.

**Acceptance Criteria:**

**Given** the cron tool is called
**When** scheduling a one-time task (FireAt)
**Then** it accepts parameters: type="fire_at", time (ISO 8601), command
**And** schedules execution at specified time
**And** time must be in the future

**Given** the cron tool is called
**When** scheduling a recurring task (Interval)
**Then** it accepts parameters: type="interval", minutes (int >= 2), command
**And** schedules execution every N minutes
**And** minimum interval is 2 minutes

**Given** a FireAt job
**When** the scheduled time arrives
**Then** the command is executed
**And** output is logged
**And** job is removed after execution

**Given** an Interval job
**When** each interval period completes
**Then** the command is executed
**And** continues until cancelled
**And** tracks execution count

**Given** the cron scheduler
**When** jobs are pending
**Then** it checks every minute for due jobs
**And** executes all due jobs
**And** handles multiple concurrent jobs

**Given** job management
**When** listing scheduled jobs
**Then** it returns all active jobs
**And** includes job ID, type, next execution time
**And** includes command to be executed

**Given** job cancellation
**When** cancelling a job by ID
**Then** it removes the job from scheduler
**And** confirms cancellation
**And** prevents further executions

**Given** a job fails
**When** execution errors occur
**Then** error is logged
**And** Interval jobs continue (don't stop on failure)
**And** FireAt jobs are marked as failed

### Story 7.2: Write Memory Tool

As an agent,
I want to persist information to memory,
So that important context is preserved across conversations.

**Acceptance Criteria:**

**Given** the write_memory tool is called
**When** writing to long-term memory
**Then** it accepts parameter: content (string)
**And** appends content to MEMORY.md
**And** adds timestamp automatically

**Given** a memory write
**When** appending to MEMORY.md
**Then** content is formatted with date header
**And** uses Markdown format
**And** creates file if it doesn't exist

**Given** memory write succeeds
**When** operation completes
**Then** returns confirmation "Memory updated"
**And** includes memory file path

**Given** write_memory tool
**When** writing daily notes
**Then** agent can specify: type="daily", content
**And** creates file: YYYY-MM-DD.md
**And** stores in workspace/memory/ directory

**Given** memory file growth
**When** MEMORY.md exceeds 1MB
**Then** system logs warning about file size
**And** suggests memory maintenance
**And** continues operating normally

### Story 7.3: Skill Management Tools

As a user,
I want to create and manage reusable skills,
So that I can extend the agent's capabilities.

**Acceptance Criteria:**

**Given** the create_skill tool is called
**When** creating a new skill
**Then** it accepts parameters: name, description, parameters (schema), implementation
**And** creates directory: `~/.miniclaw/workspace/skills/{name}/`
**And** creates SKILL.md with metadata

**Given** a skill creation
**When** SKILL.md is written
**Then** it includes skill name and description
**And** includes parameter definitions (JSON schema)
**And** includes implementation instructions
**And** uses Markdown format

**Given** the list_skills tool is called
**When** listing available skills
**Then** it scans workspace/skills/ directory
**And** returns array of skill objects
**And** includes name and description for each

**Given** the read_skill tool is called
**When** reading a skill
**Then** it accepts parameter: name
**And** returns full SKILL.md content
**And** returns error if skill doesn't exist

**Given** the delete_skill tool is called
**When** deleting a skill
**Then** it accepts parameter: name
**And** removes the skill directory
**And** confirms deletion
**And** prevents deletion of built-in skills

**Given** skill name validation
**When** creating or accessing skills
**Then** name must be snake_case
**And** must be unique
**And** cannot conflict with built-in tools


## Epic 8: Memory System

Le système peut stocker et récupérer la mémoire court/long terme

### Story 8.1: Short-Term Memory (In-Memory)

As an agent,
I want to maintain recent context in memory,
So that I can reference recent information quickly.

**Acceptance Criteria:**

**Given** the agent processes messages
**When** storing short-term memory
**Then** it uses VecDeque with max 100 entries
**And** stores as in-memory structure
**And** entries are strings with timestamps

**Given** short-term memory reaches capacity
**When** 101st entry is added
**Then** oldest entry is removed (FIFO)
**And** new entry is appended
**And** size stays at 100

**Given** short-term memory access
**When** agent queries recent entries
**Then** returns all entries in chronological order
**And** includes timestamps for each
**And** returns empty array if no entries

**Given** system restart
**When** short-term memory is reinitialized
**Then** it starts empty (not persisted)
**And** builds up from new interactions

### Story 8.2: Long-Term Memory (MEMORY.md)

As a user,
I want persistent long-term memory storage,
So that important information survives restarts.

**Acceptance Criteria:**

**Given** long-term memory system
**When** initialized
**Then** it looks for `~/.miniclaw/workspace/memory/MEMORY.md`
**And** creates it with default template if missing

**Given** reading memory
**When** agent loads long-term memory
**Then** it reads entire MEMORY.md content
**And** parses Markdown structure
**And** extracts dated sections

**Given** MEMORY.md format
**When** writing new entries
**Then** appends to end of file
**And** includes date header (## YYYY-MM-DD)
**And** includes content as bullet points

**Given** memory access via CLI
**When** user runs `miniclaw memory read`
**Then** it displays MEMORY.md content
**And** formatted for terminal display
**And** paginated if very long

**Given** memory access with type
**When** user runs `miniclaw memory read today`
**Then** it filters entries from today's date
**And** displays only recent entries

**Given** memory access with type
**When** user runs `miniclaw memory read long`
**Then** it displays entire MEMORY.md
**And** shows all historical entries

### Story 8.3: Daily Notes

As a user,
I want automatic daily note organization,
So that I can track agent activities by date.

**Acceptance Criteria:**

**Given** the current date
**When** creating daily notes
**Then** filename format is YYYY-MM-DD.md
**And** stored in `~/.miniclaw/workspace/memory/`

**Given** writing to daily notes
**When** via write_memory tool with type="daily"
**Then** it creates/opens today's file
**And** appends content with timestamp

**Given** reading recent memories
**When** user runs `miniclaw memory recent --days N`
**Then** it reads files from last N days
**And** combines content chronologically
**And** displays with date headers

**Given** recent memory query
**When** days parameter is not provided
**Then** defaults to 7 days
**And** shows past week of notes

**Given** memory cleanup
**When** files are older than 30 days
**Then** system may archive them (optional)
**And** always preserves current month

### Story 8.4: Memory Ranker

As an agent,
I want to find relevant memories by query,
So that I can retrieve pertinent past information.

**Acceptance Criteria:**

**Given** the memory ranker
**When** searching with query
**Then** it uses keyword matching (simple ranker)
**And** compares query words against memory content

**Given** a search query
**When** ranking memories
**Then** it counts keyword matches
**And** ranks by match count (higher = more relevant)
**And** returns top N results (default 5)

**Given** user searches memory
**When** running `miniclaw memory rank -q "query"`
**Then** it searches MEMORY.md
**And** searches recent daily notes
**And** displays ranked results

**Given** search results
**When** displaying to user
**Then** includes relevance score
**And** includes memory excerpt
**And** includes date/source

**Given** no matches found
**When** searching memories
**Then** returns empty results
**And** suggests broader search terms

**Given** ranker limitations (MVP)
**When** searching
**Then** uses simple keyword matching (not semantic)
**And** no LLM involvement for ranking
**And** future versions may use LLM-based ranking


## Epic 9: Session Management

Le système gère les sessions utilisateur avec persistance automatique

### Story 9.1: Session Manager Core

As a user,
I want robust session management,
So that my conversations are isolated and persisted properly.

**Acceptance Criteria:**

**Given** the SessionManager
**When** initialized
**Then** it uses Arc<RwLock<HashMap>> for storage
**And** key is session_id (String)
**And** value is Session struct

**Given** creating a session
**When** new conversation starts
**Then** it generates session_id: "{channel}_{chat_id}"
**And** records created_at timestamp
**And** initializes empty messages vector

**Given** retrieving a session
**When** by session_id
**Then** it acquires read lock briefly
**And** clones session data
**And** releases lock immediately

**Given** updating a session
**When** adding messages
**Then** it acquires write lock briefly
**And** updates messages vector
**And** updates last_accessed timestamp
**And** releases lock immediately

**Given** concurrent access
**When** multiple threads access sessions
**Then** RwLock prevents data races
**And** multiple readers allowed simultaneously
**And** writers get exclusive access

**Given** lock scope
**When** any operation
**Then** lock is held minimally (clone, release, process)
**And** never held during I/O operations

### Story 9.2: Auto-Persistence

As a user,
I want automatic session saving,
So that I don't lose conversation history on crashes.

**Acceptance Criteria:**

**Given** the gateway is running
**When** initialized
**Then** it spawns persistence background task
**And** task runs every 30 seconds

**Given** persistence loop
**When** 30 second interval triggers
**Then** it acquires read lock on all sessions
**And** serializes each session to JSON
**And** writes to `~/.miniclaw/workspace/sessions/`
**And** releases lock

**Given** session file naming
**When** saving sessions
**Then** filename is `{session_id}.json`
**And** example: `telegram_123456789.json`

**Given** persistence failure
**When** disk is full or permission denied
**Then** error is logged with level ERROR
**And** operation continues (doesn't crash)
**And** retries on next interval

**Given** graceful shutdown
**When** SIGTERM is received
**Then** persistence task completes current save
**And** all sessions are flushed to disk
**And** then system shuts down

### Story 9.3: Session Cleanup

As a system administrator,
I want automatic cleanup of old sessions,
So that disk space doesn't grow indefinitely.

**Acceptance Criteria:**

**Given** session TTL policy
**When** sessions are tracked
**Then** TTL is 30 days of inactivity
**And** last_accessed timestamp is used

**Given** cleanup task
**When** running daily
**Then** it scans all session files
**And** checks last_accessed for each
**And** deletes files older than 30 days

**Given** session access
**When** user sends message
**Then** last_accessed is updated to now
**And** session TTL is effectively reset

**Given** active sessions
**When** cleanup runs
**Then** recently accessed sessions are preserved
**And** only inactive sessions are removed

**Given** cleanup logging
**When** sessions are deleted
**Then** INFO level log shows count deleted
**And** shows total space freed


## Epic 10: Telegram Channel Integration

Le système peut communiquer via Telegram

### Story 10.1: Telegram Bot Adapter

As a user,
I want to communicate with miniclaw via Telegram,
So that I can interact from my phone anywhere.

**Acceptance Criteria:**

**Given** Telegram bot token is configured
**When** the gateway starts
**Then** it initializes teloxide bot client
**And** validates token format
**And** logs "Telegram channel connected"

**Given** bot initialization
**When** token is invalid
**Then** it logs ERROR "Invalid Telegram token"
**And** suggests checking @BotFather
**And** continues without Telegram (other channels may work)

**Given** the Telegram adapter
**When** started by gateway
**Then** it registers with Chat Hub
**And** receives outbound channel sender
**And** spawns inbound message handler

**Given** message receiving
**When** Telegram sends update
**Then** bot uses long-polling (30s timeout)
**And** handles Update::Message events
**And** ignores other update types (MVP)

**Given** inbound message processing
**When** text message received
**Then** it creates InboundMessage struct
**And** extracts chat_id from message.chat.id
**And** extracts content from message.text
**And** sends to Chat Hub inbound channel

**Given** outbound message sending
**When** Chat Hub sends OutboundMessage
**Then** bot calls send_message API
**And** uses chat_id from message
**And** uses text from message content

### Story 10.2: Telegram Whitelist

As a user,
I want to restrict who can interact with my agent,
So that only authorized users can access it.

**Acceptance Criteria:**

**Given** whitelist configuration
**When** configured in config.json
**Then** it has field: `allow_from: [123456789, 987654321]`
**And** contains array of allowed Telegram user IDs

**Given** incoming message
**When** checking whitelist
**Then** it extracts user_id from message.from.id
**And** checks if in allow_from list

**Given** user is whitelisted
**When** message passes check
**Then** it is processed normally
**And** routed to agent

**Given** user is NOT whitelisted
**When** message fails check
**Then** message is silently dropped
**And** not sent to agent
**And** DEBUG log shows "Message from non-whitelisted user {id}"

**Given** empty whitelist
**When** allow_from is [] or missing
**Then** all messages are rejected (secure by default)
**And** logs warn "Whitelist empty, no users allowed"

**Given** whitelist management
**When** onboarding runs
**Then** it asks for user ID
**And** adds it to allow_from automatically
**And** explains how to add more users later

### Story 10.3: Gateway Daemon Mode

As a user,
I want miniclaw to run continuously as a daemon,
So that my agent is always available.

**Acceptance Criteria:**

**Given** the gateway command
**When** I run `miniclaw gateway`
**Then** it loads configuration
**And** initializes all channels (Telegram)
**And** starts Chat Hub
**And** enters daemon mode

**Given** daemon is running
**When** processing messages
**Then** it maintains active connections
**And** handles concurrent conversations
**And** keeps sessions in memory

**Given** daemon shutdown
**When** receiving SIGTERM (Ctrl+C)
**Then** it initiates graceful shutdown
**And** stops accepting new messages
**And** completes processing current messages
**And** saves all sessions
**And** exits cleanly

**Given** daemon errors
**When** unexpected error occurs
**Then** it logs ERROR with details
**And** attempts to recover if possible
**And** continues running (unless fatal)

**Given** daemon monitoring
**When** running under systemd or docker
**Then** it supports restart policies
**And** exits with appropriate codes
**And** logs startup/shutdown events


## Epic 11: System Monitoring & Reliability

Le système est observable et résilient

### Story 11.1: Structured Logging System

As a developer,
I want comprehensive logging,
So that I can debug and monitor system behavior.

**Acceptance Criteria:**

**Given** the logging system
**When** initialized at startup
**Then** it uses tracing crate for structured logging
**And** configures subscriber with formatting

**Given** log levels
**When** logging events
**Then** supports: ERROR, WARN, INFO, DEBUG, TRACE
**And** default level is INFO

**Given** ERROR level
**When** used
**Then** logs failures requiring intervention
**And** examples: crash risks, data loss, auth failures

**Given** WARN level
**When** used
**Then** logs unexpected but handled situations
**And** examples: rate limits, retries, deprecated usage

**Given** INFO level
**When** used
**Then** logs important lifecycle events
**And** examples: startup, config loaded, connection established

**Given** DEBUG level
**When** used (verbose mode)
**Then** logs detailed operation info
**And** examples: tool execution, context assembly, API calls

**Given** TRACE level
**When** used (very verbose)
**Then** logs extremely detailed data
**And** examples: serialization, raw API responses

**Given** log format
**When** writing to stderr
**Then** includes timestamp (ISO 8601)
**And** includes level
**And** includes target module
**And** includes message

**Given** structured logging
**When** including fields
**Then** uses tracing key-value pairs
**And** example: `info!(user_id = %id, "Message received")`

**Given** secret protection
**When** logging configuration or API data
**Then** API keys are NEVER logged
**And** tokens are NEVER logged
**And** passwords are NEVER logged
**And** only existence is logged: "API key configured: true"

### Story 11.2: Output Stream Management

As a user,
I want clear separation of output and logs,
So that I can parse results programmatically.

**Acceptance Criteria:**

**Given** command execution
**When** producing output
**Then** normal results go to stdout
**And** logs go to stderr

**Given** interactive commands
**When** running `miniclaw version`
**Then** version string goes to stdout
**And** no stderr output on success

**Given** error conditions
**When** command fails
**Then** error message goes to stderr
**And** exit code is non-zero
**And** stdout may be empty

**Given** verbose mode
**When** enabled with --verbose
**Then** DEBUG logs go to stderr
**And** command output still goes to stdout
**And** streams are properly interleaved

**Given** piping commands
**When** user runs `miniclaw agent -m "hi" | grep something`
**Then** only stdout is piped
**And** stderr is shown on terminal

### Story 11.3: Performance Metrics

As a user,
I want visibility into system performance,
So that I can verify resource usage claims.

**Acceptance Criteria:**

**Given** the agent command
**When** executed with performance tracking
**Then** it measures: startup time, execution time, memory usage

**Given** startup time
**When** command begins
**Then** measured from process start to ready state
**And** logged as "Startup: Xms"

**Given** response time
**When** processing messages
**Then** measured from receive to reply sent
**And** target is < 2 seconds (95th percentile)

**Given** memory tracking
**When** gateway is running
**Then** logs current RSS memory periodically
**And** target is < 30MB idle
**And** logs warning if threshold exceeded

**Given** binary size
**When** built with release profile
**Then** measured with `strip` command
**And** target is < 15MB
**And** CI validates this on each build

**Given** performance monitoring
**When** verbose mode is enabled
**Then** shows detailed timings per component
**And** shows memory deltas
**And** helps identify bottlenecks

### Story 11.4: Error Handling and Reliability

As a user,
I want graceful error handling,
So that the system is reliable and never loses data.

**Acceptance Criteria:**

**Given** user input errors
**When** invalid command or parameters
**Then** system displays helpful error message
**And** suggests correct usage
**And** exits with code 1 or 2
**And** NEVER panics

**Given** unexpected errors
**When** internal error occurs
**Then** error is logged with full context
**And** operation may be retried if appropriate
**And** system continues running if possible

**Given** graceful degradation
**When** non-critical component fails
**Then** system logs the issue
**And** continues with reduced functionality
**And** example: if Telegram fails, other channels still work

**Given** data integrity
**When** sessions are persisted
**Then** atomic write pattern is used
**And** writes to temp file first
**And** renames to final name (atomic)
**And** prevents corruption on crash

**Given** recovery from crashes
**When** system restarts after failure
**Then** it loads last known good sessions
**And** may lose only last 30s of data (acceptable)
**And** continues operation normally

**Given** signal handling
**When** SIGTERM is received
**Then** system initiates graceful shutdown
**And** completes current operations
**And** flushes all data to disk
**And** exits cleanly

**Given** panic prevention
**When** any operation
**Then** all unwrap() calls are avoided
**And** proper error handling with ? operator
**And** no unsafe code that could segfault

