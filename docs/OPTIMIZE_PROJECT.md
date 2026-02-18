# Plan d'Optimisation - miniclaw

> Audit complet du code - Février 2026  
> Statistiques: 73 fichiers Rust, ~23 500 lignes de code, 17 fichiers de tests

---

## Table des matières

1. [Actions immédiates (P0)](#1-actions-immédiates-p0)
2. [Code dupliqué à consolider (P1)](#2-code-dupliqué-à-consolider-p1)
3. [Code mort à supprimer (P2)](#3-code-mort-à-supprimer-p2)
4. [Optimisations async/perf (P1)](#4-optimisations-asyncperf-p1)
5. [Abstractions à créer (P2)](#5-abstractions-à-créer-p2)
6. [Tests à consolider (P1)](#6-tests-à-consolider-p1)
7. [Couverture manquante (P3)](#7-couverture-manquante-p3)
8. [Dépendances à nettoyer (P2)](#8-dépendances-à-nettoyer-p2)
9. [Checklist d'implémentation](#9-checklist-dimplémentation)
10. [Gains estimés](#10-gains-estimés)
11. [Notes de mise en œuvre](#11-notes-de-mise-en-œuvre)

---

## 1. Actions immédiates (P0)

### 1.1 Supprimer les tests dupliqués

**Fichier:** `tests/version_tests.rs`

Ce fichier duplique entièrement `tests/cli_tests.rs`. Supprimer le fichier entier.

```bash
rm tests/version_tests.rs
```

**Impact:** -50 lignes, suppression de confusion

---

### 1.2 Extraire la blacklist de commandes

**Fichiers concernés:**
- `src/agent/tools/exec.rs:21-23`
- `src/agent/tools/spawn.rs:19-21`

**Créer:** `src/agent/tools/security.rs`

```rust
//! Shared security utilities for tool execution

/// Commands that are blacklisted from execution for security reasons
pub const COMMAND_BLACKLIST: &[&str] = &[
    "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo",
];

/// Check if a command is blacklisted
pub fn is_command_blacklisted(command: &str) -> bool {
    let base_cmd = std::path::Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command)
        .to_lowercase();

    COMMAND_BLACKLIST.contains(&base_cmd.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blacklist_detection() {
        assert!(is_command_blacklisted("rm"));
        assert!(is_command_blacklisted("/usr/bin/sudo"));
        assert!(is_command_blacklisted("DD"));
        assert!(!is_command_blacklisted("ls"));
    }
}
```

**Modifier:** `src/agent/tools/mod.rs` pour inclure le module

```rust
pub mod security;
```

**Modifier:** `src/agent/tools/exec.rs` et `spawn.rs`

```rust
use crate::agent::tools::security::is_command_blacklisted;

// Remplacer la fonction is_blacklisted locale par l'appel:
if is_command_blacklisted(command) { ... }
```

---

### 1.3 Corriger l'anti-pattern `RwLock<mpsc::Receiver>`

**Fichier:** `src/chat/hub.rs:30-32`

**Problème:** `RwLock<mpsc::Receiver>` force un write-lock pour recevoir, bloquant tous les lecteurs.

**Solution proposée:** Restructurer pour single-ownership du receiver

```rust
// AVANT
pub struct ChatHub {
    inbound_rx: Arc<RwLock<mpsc::Receiver<InboundMessage>>>,
    outbound_rx: Arc<RwLock<mpsc::Receiver<OutboundMessage>>>,
    // ...
}

// APRÈS - Option A: Mutex (si partage nécessaire)
pub struct ChatHub {
    inbound_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<InboundMessage>>>,
    outbound_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<OutboundMessage>>>,
    // ...
}

// APRÈS - Option B: Single owner (recommandé si possible)
pub struct ChatHub {
    inbound_rx: mpsc::Receiver<InboundMessage>,
    outbound_rx: mpsc::Receiver<OutboundMessage>,
    // ...
}
```

---

### 1.4 Optimiser le calcul de score mémoire

**Fichier:** `src/memory/ranker.rs:98-104`

**Problème:** `content.to_lowercase()` crée une nouvelle allocation pour chaque entrée scorée.

```rust
// AVANT
pub fn calculate_score(content: &str, query_tokens: &[String]) -> usize {
    let content_lower = content.to_lowercase();  // Allocation à chaque appel
    query_tokens
        .iter()
        .filter(|token| content_lower.contains(token.as_str()))
        .count()
}

// APRÈS - Recevoir le lowercase déjà calculé
pub fn calculate_score(content_lower: &str, query_tokens: &[String]) -> usize {
    query_tokens
        .iter()
        .filter(|token| content_lower.contains(token.as_str()))
        .count()
}

// Dans les appelants, pré-calculer une seule fois par entrée:
for entry in entries {
    let content_lower = entry.content.to_lowercase();
    let score = Self::calculate_score(&content_lower, query_tokens);
    // ...
}
```

---

## 2. Code dupliqué à consolider (P1)

### 2.1 Constante `WORKSPACE_FILES` dupliquée

**Fichiers:**
- `src/workspace/templates.rs:581-587`
- `src/workspace/mod.rs:32-38`

**Action:** Supprimer la définition dans `mod.rs`, utiliser `templates::WORKSPACE_FILES`

```rust
// src/workspace/mod.rs
use templates::WORKSPACE_FILES;  // Utiliser la version de templates

// Supprimer la constante locale WORKSPACE_FILES
```

---

### 2.2 Structs d'entrée mémoire similaires

**Fichiers:**
- `src/memory/long_term.rs:52-58` — `LongTermMemoryEntry`
- `src/memory/short_term.rs:16-22` — `MemoryEntry`
- `src/memory/daily_notes.rs:20-26` — `DailyNoteEntry`

**Proposition:** Unifier dans `src/memory/types.rs`

```rust
/// Unified memory entry structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEntry {
    /// The content of the memory entry
    pub content: String,
    /// When the entry was created (ISO 8601 UTC)
    pub timestamp: DateTime<Utc>,
    /// Optional source identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<MemorySource>,
}

impl MemoryEntry {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
            source: None,
        }
    }

    pub fn with_source(mut self, source: MemorySource) -> Self {
        self.source = Some(source);
        self
    }
}
```

---

### 2.3 Configs providers similaires

**Fichier:** `src/providers/factory.rs`

Les structs `OpenRouterConfig`, `OpenAiConfig`, `KimiConfig` sont quasi-identiques avec les
mêmes champs et les mêmes méthodes `validate()`.

**Proposition:** Créer un trait commun

```rust
/// Trait for API-key-based provider configurations
pub trait ApiKeyProviderConfig {
    fn api_key(&self) -> &str;
    fn base_url(&self) -> &str;
    fn default_model(&self) -> &str;
    fn timeout_seconds(&self) -> u64;
    fn organization_id(&self) -> Option<&str> { None }

    fn validate(&self, provider_name: &str) -> Result<(), ProviderError> {
        if self.api_key().is_empty() {
            return Err(ProviderError::config(format!(
                "{} API key is required", provider_name
            )));
        }
        if self.base_url().is_empty() {
            return Err(ProviderError::config(format!(
                "{} base URL cannot be empty", provider_name
            )));
        }
        if self.default_model().is_empty() {
            return Err(ProviderError::config(format!(
                "{} default model cannot be empty", provider_name
            )));
        }
        Ok(())
    }
}
```

---

### 2.4 Affichage configuration tripliqué

**Fichier:** `src/config/onboard.rs` (lignes 144-162, 417-426, 437-455)

**Créer des fonctions helpers:**

```rust
/// Format configuration for display
fn format_config_summary(config: &Config, indent: &str) -> String {
    let mut output = String::new();

    if let Some(provider_config) = &config.provider_config {
        output.push_str(&format!("{}Provider: {}\n", indent, provider_config.provider_type()));
        output.push_str(&format!("{}Model: {}\n", indent, provider_config.default_model()));
    } else {
        output.push_str(&format!("{}Provider: (not set)\n", indent));
    }

    if let Some(token) = &config.telegram_token {
        output.push_str(&format!("{}Telegram Bot: {}\n", indent, mask_secret(token)));
    } else {
        output.push_str(&format!("{}Telegram Bot: (not set)\n", indent));
    }

    output.push_str(&format!(
        "{}Whitelisted Users: {}\n",
        indent,
        format_user_ids(&config.allow_from)
    ));

    output
}

fn format_user_ids(user_ids: &[i64]) -> String {
    if user_ids.is_empty() {
        "(empty - secure by default, no access)".to_string()
    } else {
        user_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
```

---

### 2.5 Mapping erreur validation chemin

**Fichiers:**
- `src/agent/tools/filesystem.rs:69-88`
- `src/agent/tools/exec.rs:97-123`
- `src/agent/tools/spawn.rs:97-122`

**Ajouter dans `src/agent/tools/types.rs`:**

```rust
use crate::utils::paths::PathValidationError;

/// Convert a path validation error into a tool error
pub fn map_path_validation_error(e: PathValidationError, tool_name: &str) -> ToolError {
    match e {
        PathValidationError::OutsideBaseDirectory(path) => ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            message: format!("Path '{}' is outside the allowed base directory", path),
        },
        PathValidationError::SystemPathBlocked(path) => ToolError::PermissionDenied {
            tool: tool_name.to_string(),
            message: format!("Access to system path '{}' is not allowed", path),
        },
        PathValidationError::CanonicalizationFailed { path, source } => {
            ToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                message: format!("Failed to resolve path '{}': {}", path, source),
            }
        }
        PathValidationError::InvalidBaseDirectory(msg) => ToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            message: msg,
        },
    }
}
```

---

### 2.6 Pattern de permissions Unix répété 5 fois

**Fichiers:**
- `src/skills/loader.rs:43-49`
- `src/workspace/mod.rs:126-133, 152-158, 200-207, 350-355`
- `src/session/persistence.rs:104-113, 238-245`

**Créer `src/utils/fs.rs`:**

```rust
//! Filesystem utilities

use std::path::Path;

/// Set file/directory permissions on Unix systems (no-op on other platforms)
#[cfg(unix)]
pub async fn set_permissions_unix(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).await
}

#[cfg(not(unix))]
pub async fn set_permissions_unix(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Common Unix permission modes
pub mod permissions {
    pub const DIR_755: u32 = 0o755;   // rwxr-xr-x
    pub const FILE_600: u32 = 0o600;  // rw-------
    pub const FILE_644: u32 = 0o644;  // rw-r--r--
}
```

**Ajouter dans `src/utils/mod.rs`:**

```rust
pub mod fs;
```

---

### 2.7 Pattern de chargement fichiers workspace répété 5 fois

**Fichier:** `src/workspace/mod.rs:249-302`

**Extraire un helper:**

```rust
fn load_workspace_file(workspace_path: &Path, filename: &str) -> Option<String> {
    let path = workspace_path.join(filename);
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => Some(content),
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to read {}",
                    filename
                );
                None
            }
        }
    } else {
        None
    }
}

// Utilisation dans load_workspace_context:
context.soul = load_workspace_file(workspace_path, "SOUL.md").unwrap_or_default();
context.agents = load_workspace_file(workspace_path, "AGENTS.md").unwrap_or_default();
context.user = load_workspace_file(workspace_path, "USER.md").unwrap_or_default();
context.tools = load_workspace_file(workspace_path, "TOOLS.md").unwrap_or_default();
context.heartbeat = load_workspace_file(workspace_path, "HEARTBEAT.md").unwrap_or_default();
```

---

### 2.8 Initialisation SkillsManager répétée dans skill.rs

**Fichier:** `src/agent/tools/skill.rs` (lignes 145-154, 252-261, 343-351, 432-440)

```rust
// Pattern répété 4 fois:
let skills_manager = SkillsManager::new(self.workspace_path.clone());
skills_manager
    .load_skills()
    .await
    .map_err(|e| ToolError::ExecutionFailed {
        tool: self.name().to_string(),
        message: format!("Failed to load skills: {}", e),
    })?;

// Extraire un helper sur un trait commun:
trait SkillToolHelper {
    fn workspace_path(&self) -> &Path;

    async fn load_skills_manager(&self) -> ToolResult<SkillsManager> {
        let manager = SkillsManager::new(self.workspace_path().to_path_buf());
        manager
            .load_skills()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "skill_tool".to_string(),
                message: format!("Failed to load skills: {}", e),
            })?;
        Ok(manager)
    }
}
```

---

### 2.9 Pattern de buffer overflow dupliqué dans ChatHub

**Fichier:** `src/chat/hub.rs:108-150`

```rust
// AVANT - logique dupliquée dans send_inbound et send_outbound
Err(mpsc::error::TrySendError::Full(msg)) => {
    tracing::warn!("Inbound buffer full, dropping oldest message");
    if let Ok(mut rx) = self.inbound_rx.try_write() {
        let _ = rx.try_recv();
    }
    self.inbound_tx.send(msg).await
        .map_err(|e| ChatError::SendError(e.to_string()))
}

// APRÈS - helper générique
async fn drain_and_resend<T>(
    rx: &Arc<RwLock<mpsc::Receiver<T>>>,
    tx: &mpsc::Sender<T>,
    msg: T,
    direction: &str,
) -> Result<()> {
    tracing::warn!("{} buffer full, dropping oldest message", direction);
    if let Ok(mut rx) = rx.try_write() {
        let _ = rx.try_recv();
    }
    tx.send(msg).await.map_err(|e| ChatError::SendError(e.to_string()))
}
```

---

### 2.10 Extraction session ID dupliquée

**Fichiers:**
- `src/session/persistence.rs:170-174`
- `src/session/cleanup.rs:82-86`

**Ajouter dans `src/session/types.rs`:**

```rust
/// Extract session ID from a session file path
pub fn session_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Check whether a path is a session JSON file
pub fn is_session_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("json")
}
```

---

## 3. Code mort à supprimer (P2)

### 3.1 Liste complète des suppressions

| Fichier                  | Lignes  | Code à supprimer              | Raison                    |
| ------------------------ | ------- | ----------------------------- | ------------------------- |
| `config/onboard.rs`        | 291-296 | `fn prompt_api_key()`           | Explicitement remplacée   |
| `utils/error.rs`           | 62-75   | `fn sanitize_path()`            | Jamais appelée            |
| `chat/hub.rs`              | 19-20   | `ChatError::IoError`            | Aucun chemin ne l'utilise |
| `channels/telegram.rs`     | 26-27   | `TelegramError::NotInitialized` | Seulement dans les tests  |
| `gateway.rs`               | 379-385 | `fn trigger_persistence()`      | Jamais appelée            |
| `agent/tools/metrics.rs`   | 10      | `TARGET_RESPONSE_TIME_MS`       | Doublon de agent_loop.rs  |
| `skills/manager.rs`        | 352-359 | Paramètre `_built_in_tools`     | Déprécié, jamais utilisé  |
| `providers/openai.rs`      | 574-578 | `fn timeout_seconds()` hardcodé | Retourne 30 en dur        |

### 3.2 Exports inutilisés dans `session/mod.rs`

```rust
// AVANT
pub use cleanup::{CLEANUP_INTERVAL_SECS, Cleanup, CleanupResult, SESSION_TTL_DAYS};
pub use manager::{PERSISTENCE_INTERVAL_SECS, SessionManager};
pub use persistence::Persistence;
pub use types::{MAX_MESSAGES, Message, Session, ToolCall};

// APRÈS — Ne garder que ce qui est consommé hors du module
pub use manager::SessionManager;
pub use types::{Message, Session, ToolCall};
```

---

## 4. Optimisations async/perf (P1)

### 4.1 Double acquisition de lock dans ToolRegistry

**Fichier:** `src/agent/tools/mod.rs:407-426`

```rust
// AVANT — deux read locks successifs
pub async fn execute_tool_with_timeout(...) -> types::ToolResult<String> {
    let schema = {
        let tools = self.tools.read().await;
        let tool = tools.get(name)?;
        tool.parameters()
    };
    validate_args_against_schema(&args, &schema, name)?;

    let tools = self.tools.read().await;  // Seconde acquisition inutile
    let tool = tools.get(name)?;
    tokio::time::timeout(timeout, tool.execute(args, ctx)).await
}

// APRÈS — une seule acquisition
#[allow(clippy::await_holding_lock)]
pub async fn execute_tool_with_timeout(...) -> types::ToolResult<String> {
    let tools = self.tools.read().await;
    let tool = tools.get(name).ok_or_else(|| ToolError::NotFound(name.to_string()))?;
    let schema = tool.parameters();
    validate_args_against_schema(&args, &schema, name)?;

    tokio::time::timeout(timeout, tool.execute(args, ctx)).await
}
```

---

### 4.2 I/O bloquant dans les fonctions async

**Fichier:** `src/memory/long_term.rs:163, 306`

```rust
// AVANT — Path::exists() est bloquant
if !self.file_path.exists() {

// APRÈS — utiliser tokio::fs
match tokio::fs::metadata(&self.file_path).await {
    Ok(_) => { /* le fichier existe */ }
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => { /* n'existe pas */ }
    Err(e) => return Err(MemoryError::storage_failed("check file existence", e)),
}
```

**Même correction dans `src/agent/tools/skill.rs`** pour les canonicalisations bloquantes.

---

### 4.3 Cache mémoire avec clone complet

**Fichier:** `src/memory/long_term.rs:299-304`

```rust
// AVANT — clone tout le Vec à chaque hit de cache
pub struct LongTermMemory {
    cache: Arc<Mutex<Option<Vec<MemorySection>>>>,
}
// ...
return Ok(sections.clone());  // Clone coûteux

// APRÈS — Arc pour partager sans copier
pub struct LongTermMemory {
    cache: Arc<Mutex<Option<Arc<Vec<MemorySection>>>>>,
}

pub async fn read_all(&self) -> Result<Arc<Vec<MemorySection>>> {
    let cache = self.cache.lock().await;
    if let Some(ref sections) = *cache {
        return Ok(Arc::clone(sections));  // Clone de l'Arc seulement (1 atomique)
    }
    drop(cache);
    // ... charger, wrapper dans Arc::new(), mettre en cache
}
```

---

### 4.4 Sauvegardes de sessions séquentielles

**Fichier:** `src/session/manager.rs:117-134`

```rust
// AVANT — séquentiel
for session in &sessions {
    if let Err(e) = self.persistence.save_session(session).await {
        error!(session_id = %session.session_id, error = %e, "Failed to save session");
    }
}

// APRÈS — parallèle avec join_all
use futures::future::join_all;

let persistence = Arc::clone(&self.persistence);
let save_futures: Vec<_> = sessions
    .iter()
    .map(|session| {
        let p = Arc::clone(&persistence);
        let s = session.clone();
        async move { (s.session_id.clone(), p.save_session(&s).await) }
    })
    .collect();

for (session_id, result) in join_all(save_futures).await {
    if let Err(e) = result {
        error!(session_id = %session_id, error = %e, "Failed to save session");
    }
}
```

---

### 4.5 Tracking "dirty" pour éviter les sauvegardes inutiles

**Fichier:** `src/session/manager.rs`

Actuellement, **toutes** les sessions sont écrites sur disque toutes les 30 secondes, même sans modification.

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    dirty_sessions: Arc<RwLock<HashSet<String>>>,  // NOUVEAU
    persistence: Arc<Persistence>,
}

impl SessionManager {
    /// Marquer une session comme modifiée lors de l'ajout d'un message
    pub async fn add_message(&self, session_id: &str, message: Message) -> Result<()> {
        // ... logique existante ...
        self.dirty_sessions.write().await.insert(session_id.to_string());
        Ok(())
    }

    /// Sauvegarder uniquement les sessions modifiées
    pub async fn save_dirty_sessions(&self) -> Result<()> {
        let dirty = {
            let mut guard = self.dirty_sessions.write().await;
            std::mem::take(&mut *guard)
        };
        if dirty.is_empty() { return Ok(()); }

        let sessions = self.sessions.read().await;
        let to_save: Vec<_> = dirty
            .iter()
            .filter_map(|id| sessions.get(id))
            .cloned()
            .collect();
        drop(sessions);

        // Sauvegarder en parallèle
        let futures: Vec<_> = to_save
            .iter()
            .map(|s| self.persistence.save_session(s))
            .collect();
        join_all(futures).await;
        Ok(())
    }
}
```

---

### 4.6 Lock maintenu pendant l'envoi dans ChatHub

**Fichier:** `src/chat/hub.rs:176-207`

```rust
// AVANT — read lock maintenu pendant toute la boucle de retry
pub async fn route_outbound(&self, message: OutboundMessage) -> Result<()> {
    let channels = self.channels.read().await;  // Lock tenu pendant retry
    // ... boucle de retry avec le lock ...
}

// APRÈS — cloner le sender et relâcher le lock avant d'envoyer
pub async fn route_outbound(&self, message: OutboundMessage) -> Result<()> {
    let sender = {
        let channels = self.channels.read().await;
        channels.get(&message.channel).cloned()
    };  // Lock relâché ici

    match sender {
        Some(tx) => { /* retry loop sans lock */ }
        None => Err(ChatError::ChannelNotFound(message.channel)),
    }
}
```

---

## 5. Abstractions à créer (P2)

### 5.1 Helpers d'extraction de paramètres outils

**Fichier:** `src/agent/tools/types.rs`

Ce pattern est répété 15+ fois dans tous les fichiers outils.

```rust
use serde_json::Value;
use std::collections::HashMap;

/// Extract a required string parameter from tool arguments
pub fn extract_string_arg<'a>(
    args: &'a HashMap<String, Value>,
    tool_name: &str,
    param_name: &str,
) -> ToolResult<&'a str> {
    args.get(param_name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Missing required parameter '{}'", param_name),
        })
}

/// Extract an optional string parameter
pub fn extract_optional_string<'a>(
    args: &'a HashMap<String, Value>,
    param_name: &str,
) -> Option<&'a str> {
    args.get(param_name).and_then(|v| v.as_str())
}

/// Extract a required integer parameter
pub fn extract_int_arg(
    args: &HashMap<String, Value>,
    tool_name: &str,
    param_name: &str,
) -> ToolResult<i64> {
    args.get(param_name)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Missing or invalid integer parameter '{}'", param_name),
        })
}

/// Extract a required boolean parameter
pub fn extract_bool_arg(
    args: &HashMap<String, Value>,
    tool_name: &str,
    param_name: &str,
) -> ToolResult<bool> {
    args.get(param_name)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Missing or invalid boolean parameter '{}'", param_name),
        })
}
```

**Utilisation:**

```rust
// AVANT (15+ occurrences)
let command = args.get("command").and_then(|v| v.as_str()).ok_or_else(|| {
    ToolError::InvalidArguments {
        tool: self.name().to_string(),
        message: "Missing required parameter 'command'".to_string(),
    }
})?;

// APRÈS
let command = extract_string_arg(&args, self.name(), "command")?;
```

---

### 5.2 Helpers de réponse JSON

**Fichier:** `src/agent/tools/types.rs`

```rust
/// Create a success JSON response
pub fn success_response(message: impl Into<String>) -> ToolResult<String> {
    Ok(serde_json::json!({
        "success": true,
        "message": message.into()
    }).to_string())
}

/// Create a success response with data payload
pub fn data_response<T: serde::Serialize>(data: &T) -> ToolResult<String> {
    Ok(serde_json::json!({
        "success": true,
        "data": data
    }).to_string())
}

/// Create an error response (non-fatal, returned as Ok)
pub fn error_response(message: impl Into<String>) -> ToolResult<String> {
    Ok(serde_json::json!({
        "success": false,
        "error": message.into()
    }).to_string())
}
```

---

### 5.3 Helper d'erreur mémoire

**Fichier:** `src/memory/types.rs`

Ce pattern apparaît 17 fois :

```rust
.map_err(|e| MemoryError::StorageFailed {
    operation: "create memory directory".to_string(),
    source: e,
})?;
```

**Ajouter à `MemoryError`:**

```rust
impl MemoryError {
    /// Create a StorageFailed error with context
    pub fn storage_failed(operation: impl Into<String>, source: std::io::Error) -> Self {
        Self::StorageFailed {
            operation: operation.into(),
            source,
        }
    }

    /// Validate that content is not empty or whitespace-only
    pub fn validate_content(content: &str) -> Result<&str, Self> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            Err(Self::InvalidContent("Content cannot be empty".to_string()))
        } else {
            Ok(trimmed)
        }
    }
}

// Utilisation:
.map_err(|e| MemoryError::storage_failed("create memory directory", e))?;
```

---

### 5.4 Pattern de boucles de scoring dupliqué dans ranker.rs

**Fichier:** `src/memory/ranker.rs:153-166, 202-215`

```rust
// Extraire une fonction générique
fn score_section_entries(
    entries: impl Iterator<Item = (String, DateTime<Utc>)>,
    query_tokens: &[String],
    source: MemorySource,
) -> Vec<RankedMemory> {
    entries
        .filter_map(|(content, timestamp)| {
            let content_lower = content.to_lowercase();
            let score = Self::calculate_score(&content_lower, query_tokens);
            if score > 0 {
                Some(RankedMemory {
                    excerpt: Self::create_excerpt(&content),
                    content,
                    score,
                    source: source.clone(),
                    date: Some(timestamp),
                })
            } else {
                None
            }
        })
        .collect()
}
```

---

### 5.5 Macro pour les avertissements de champs dépréciés

**Fichier:** `src/config/loader.rs:138-163`

```rust
macro_rules! warn_deprecated_field {
    ($field:expr, $name:literal, $replacement:literal) => {
        if $field.is_some() {
            tracing::warn!(
                "Config file contains deprecated '{}' field. \
                 This field is ignored. Use '{}' instead. \
                 Run 'miniclaw onboard' to update your configuration.",
                $name, $replacement
            );
        }
    };
}

// Utilisation:
warn_deprecated_field!(file_config.model,         "model",         "provider_config.default_model");
warn_deprecated_field!(file_config.api_key,        "api_key",       "provider_config.api_key");
warn_deprecated_field!(file_config.provider_type,  "provider_type", "provider_config.type");
```

---

### 5.6 Méthode `set_default_model` sur `ProviderConfig`

**Fichier:** `src/config/loader.rs:84-98`

```rust
// AVANT — match répété à chaque évolution du nombre de providers
fn apply_cli_model_override(config: &mut Config, model: &str) {
    if let Some(ref mut provider_config) = config.provider_config {
        match provider_config {
            ProviderConfig::OpenRouter(cfg) => cfg.default_model = model.to_string(),
            ProviderConfig::OpenAi(cfg)     => cfg.default_model = model.to_string(),
            ProviderConfig::Kimi(cfg)       => cfg.default_model = model.to_string(),
            ProviderConfig::Ollama(cfg)     => cfg.default_model = model.to_string(),
            #[cfg(test)]
            ProviderConfig::Mock            => {}
        }
    }
}

// APRÈS — méthode sur l'enum
impl ProviderConfig {
    pub fn set_default_model(&mut self, model: impl Into<String>) {
        let model = model.into();
        match self {
            ProviderConfig::OpenRouter(cfg) => cfg.default_model = model,
            ProviderConfig::OpenAi(cfg)     => cfg.default_model = model,
            ProviderConfig::Kimi(cfg)       => cfg.default_model = model,
            ProviderConfig::Ollama(cfg)     => cfg.default_model = model,
            #[cfg(test)]
            ProviderConfig::Mock            => {}
        }
    }
}

// Appel simplifié:
if let Some(ref mut pc) = config.provider_config {
    pc.set_default_model(model);
}
```

---

### 5.7 Builder pour `AgentLoop`

**Fichier:** `src/agent/agent_loop.rs:76-165`

Quatre constructeurs quasi-identiques (`new`, `with_inbound_receiver`, `with_model`, `with_model_and_receiver`).

```rust
pub struct AgentLoopBuilder {
    chat_hub: Arc<ChatHub>,
    llm_provider: Arc<dyn LlmProvider>,
    context_builder: Arc<ContextBuilder>,
    tool_registry: Arc<ToolRegistry>,
    session_manager: Arc<RwLock<SessionManager>>,
    model: Option<String>,
    inbound_rx: Option<mpsc::Receiver<InboundMessage>>,
}

impl AgentLoopBuilder {
    pub fn new(/* required args */) -> Self { ... }
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into()); self
    }
    pub fn with_receiver(mut self, rx: mpsc::Receiver<InboundMessage>) -> Self {
        self.inbound_rx = Some(rx); self
    }
    pub fn build(self) -> AgentLoop { ... }
}
```

---

## 6. Tests à consolider (P1)

### 6.1 Créer un module de test commun

**Fichier:** `tests/common/mod.rs` (nouveau)

```rust
//! Shared test utilities

use miniclaw::session::SessionManager;
use tempfile::TempDir;

/// Create an initialized SessionManager backed by a temporary directory.
/// Returns the TempDir to keep it alive for the test duration.
pub async fn create_test_session_manager() -> (TempDir, SessionManager) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let sessions_dir = temp_dir.path().join("sessions");
    let manager = SessionManager::new(sessions_dir);
    manager.initialize().await.expect("Failed to initialize");
    (temp_dir, manager)
}

/// Run a miniclaw CLI subcommand via `cargo run` and return the full output.
pub fn run_miniclaw(args: &[&str]) -> std::process::Output {
    let mut full_args = vec!["run", "--"];
    full_args.extend_from_slice(args);
    std::process::Command::new("cargo")
        .args(&full_args)
        .output()
        .expect("Failed to execute miniclaw")
}

/// Build a minimal skill Markdown document for testing.
pub fn create_test_skill_md(name: &str, description: &str) -> String {
    format!(
        "# Skill: {name}\n\n## Description\n{description}\n\n## Parameters\n\
         - `param1` (string, required): First parameter\n\n## Usage\nExample.\n",
        name = name,
        description = description
    )
}
```

**Inclure dans chaque fichier de test qui en a besoin:**

```rust
mod common;
```

---

### 6.2 Doublons de tests à supprimer

| Test                              | Garder dans                        | Supprimer de           |
| --------------------------------- | ---------------------------------- | ---------------------- |
| `test_version_command` (et variantes) | `tests/cli_tests.rs`                 | `tests/version_tests.rs` (tout le fichier) |
| `test_gateway_command_available`    | `tests/gateway_tests.rs`             | `src/gateway.rs`         |
| `test_concurrent_session_access`    | `tests/session_integration_tests.rs` | `tests/gateway_tests.rs` |
| `test_corrupted_session_recovery`   | `tests/session_integration_tests.rs` | `tests/session_tests.rs` |
| `test_graceful_shutdown_saves_sessions` | `tests/gateway_tests.rs`         | `src/gateway.rs`         |

---

### 6.3 Remplacer les versions hardcodées

**Fichiers:** `tests/cli_tests.rs`, partout où `"0.1.0"` est utilisé

```rust
// AVANT
assert!(stdout.contains("0.1.0"));

// APRÈS
assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
```

---

## 7. Couverture manquante (P3)

### 7.1 Tests à ajouter

| Zone                    | Scénario manquant                                                          |
| ----------------------- | -------------------------------------------------------------------------- |
| `cli.rs`                  | `handle_agent` — erreur au démarrage du runtime Tokio                       |
| `cli.rs`                  | `handle_models` — erreur réseau / provider inaccessible                     |
| `cli.rs`                  | `handle_gateway` — impossibilité d'écrire le fichier PID                    |
| Gateway                 | Échec `TelegramChannel` après démarrage réussi                              |
| Gateway                 | Dépassement du seuil mémoire (lignes 239-240)                              |
| Gateway                 | Timeout de la tâche de persistence                                         |
| `memory --long`           | Chemin alternatif `long = true`                                             |
| `memory recent --days`    | Valeur > 365 (avertissement ligne 461-464)                                  |
| Signal handlers (Unix)  | Vérification que SIGTERM/SIGINT déclenchent réellement l'arrêt gracieux    |
| `--config` flag           | Chemin invalide et JSON malformé                                            |

### 7.2 Exemple de nouveau test

```rust
// tests/cli_error_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_gateway_with_invalid_config_path() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.args(["--config", "/nonexistent/path.toml", "gateway"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_version_uses_cargo_pkg_version() {
    let mut cmd = Command::cargo_bin("miniclaw").unwrap();
    cmd.arg("version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
```

---

## 8. Dépendances à nettoyer (P2)

### 8.1 Versions dupliquées détectées via `cargo tree --duplicates`

| Crate         | Versions détectées | Origine du conflit              |
| ------------- | ------------------ | --------------------------------|
| `derive_more`   | v1.0.0 + v2.1.1    | `teloxide` (v1) vs `inquire` (v2) |
| `getrandom`     | v0.2.17 + v0.4.1   | `ring` (v0.2) vs `tempfile`/`uuid` (v0.4) |
| `memchr`        | v2.8.0 (x2)        | Arbre de dépendances transitives |

### 8.2 Actions recommandées

1. Vérifier si `inquire` peut monter en version pour aligner `derive_more`
2. Évaluer la mise à jour de `teloxide` pour réduire les conflits
3. Si non résolubles, documenter explicitement dans `Cargo.toml`

```toml
# Cargo.toml — section à ajouter si nécessaire
# [patch.crates-io]
# Documenter ici toute contrainte de version forcée avec la raison
```

---

## 9. Checklist d'implémentation

### Phase 1 — Quick wins (~2h)

- [ ] Supprimer `tests/version_tests.rs`
- [ ] Supprimer les 8 éléments de code mort (section 3.1)
- [ ] Créer `src/agent/tools/security.rs` et dédupliquer la blacklist
- [ ] Optimiser `calculate_score()` dans `ranker.rs`
- [ ] Remplacer `"0.1.0"` par `env!("CARGO_PKG_VERSION")` dans les tests
- [ ] Supprimer la constante `WORKSPACE_FILES` dupliquée dans `workspace/mod.rs`

### Phase 2 — Refactoring (~4h)

- [ ] Créer `tests/common/mod.rs` avec les helpers partagés
- [ ] Ajouter `extract_string_arg` et consorts dans `agent/tools/types.rs`
- [ ] Corriger l'anti-pattern `RwLock<Receiver>` dans `chat/hub.rs`
- [ ] Optimiser le cache mémoire avec `Arc<Vec<...>>`
- [ ] Créer `src/utils/fs.rs` et centraliser les permissions Unix
- [ ] Ajouter `MemoryError::storage_failed()` helper et remplacer les 17 occurrences
- [ ] Ajouter `session_id_from_path()` et `is_session_file()` dans `session/types.rs`
- [ ] Corriger les appels `Path::exists()` bloquants en async

### Phase 3 — Architecture (~6h)

- [ ] Consolider les 3 structs d'entrée mémoire en `MemoryEntry` unifié
- [ ] Créer le trait `ApiKeyProviderConfig` et simplifier les configs providers
- [ ] Paralléliser les sauvegardes de sessions avec `join_all`
- [ ] Ajouter le tracking "dirty" pour les sessions
- [ ] Implémenter le builder `AgentLoopBuilder`
- [ ] Ajouter `ProviderConfig::set_default_model()` et supprimer le match répété
- [ ] Extraire `format_config_summary()` et `format_user_ids()` dans `onboard.rs`

### Phase 4 — Tests (~3h)

- [ ] Supprimer les tests dupliqués (section 6.2)
- [ ] Consolider les tests de session
- [ ] Ajouter les tests manquants identifiés en section 7

---

## 10. Gains estimés

| Métrique                    | Avant   | Après   | Gain          |
| --------------------------- | ------- | ------- | ------------- |
| Lignes de code source       | ~23 500 | ~22 750 | -750 (-3%)    |
| Tests dupliqués             | 8       | 0       | -100%         |
| Éléments de code mort       | 8       | 0       | -100%         |
| Occurrences pattern répété  | 15+     | 0       | -100%         |
| Perf. cache mémoire         | —       | —       | +50% (arc)    |
| Perf. sauvegarde sessions   | —       | —       | +80% (//isme) |
| Allocs. scoring mémoire     | N×      | 1×      | -80%          |

---

## 11. Notes de mise en œuvre

1. **Commencer par la Phase 1** — les quick wins ont un impact immédiat sans risque de régression
2. **Exécuter `cargo test --all`** après chaque groupe de changements
3. **Valider avec `cargo clippy --all-targets -- -D warnings`** pour chaque refactoring
4. **Vérifier la taille du binaire** après les optimisations (`./scripts/check-binary-size.sh`)
5. **Ne pas modifier les interfaces publiques** sans vérifier tous les consommateurs
6. **Documenter les breaking changes** dans `CHANGELOG.md` si nécessaire

---

*Rapport d'audit généré le 18 février 2026*
