---
title: "Connecter ChatHub à AgentLoop et ajouter mode allow-all"
slug: "connect-chathub-agentloop-allow-all"
created: "2026-02-18"
status: "completed"
stepsCompleted: [1, 2, 3, 4]
tech_stack:
  - Rust
  - tokio
  - mpsc channels
  - Arc<RwLock>
files_to_modify:
  - src/chat/hub.rs
  - src/agent/agent_loop.rs
  - src/utils/security.rs
  - src/gateway.rs
code_patterns:
  - "mpsc channels for decoupled message passing"
  - "Arc<RwLock> for shared state"
  - "tokio::select! for concurrent message handling"
  - "Secure-by-default with explicit allow-all opt-in"
test_patterns:
  - "Unit tests for SecurityWhitelist"
  - "Integration tests for message flow"
  - "Mock channel adapters for testing"
---

# Tech-Spec: Connecter ChatHub à AgentLoop et ajouter mode allow-all

**Created:** 2026-02-18

## Overview

### Problem Statement

L'architecture actuelle est cassée : ChatHub reçoit les messages entrants mais ne les transmet pas à AgentLoop. L'AgentLoop a une méthode `run()` qui tourne en boucle vide sans consommer de messages. De plus, la whitelist par défaut bloque tous les utilisateurs si aucun n'est configuré, ce qui peut être bloquant pour le développement/test.

### Solution

1. **Connecter ChatHub à AgentLoop** via un canal dédié (pattern producteur-consommateur avec mpsc)
2. **Implémenter le traitement des messages** dans AgentLoop.run() pour recevoir et traiter les messages du ChatHub
3. **Ajouter un mode "allow all"** à la whitelist avec avertissement explicite (warn log)
4. **Tests d'intégration** vérifiant le flux complet Telegram → ChatHub → AgentLoop

### Scope

**In Scope:**

- Modification de ChatHub pour émettre les messages entrants via un canal dédié vers AgentLoop
- Modification d'AgentLoop pour consommer les messages via le canal
- Implémentation du traitement de messages dans AgentLoop.run()
- Ajout du mode "allow all" (wildcard "\*") dans SecurityWhitelist avec avertissement
- Tests unitaires pour le mode allow-all
- Tests d'intégration pour le flux de messages complet

**Out of Scope:**

- Modification de la logique métier de traitement des messages (juste le plumbing)
- Changement du comportement par défaut (empty = deny all reste)
- UI/CLI pour gérer la whitelist
- Persistence des messages ou retry complexe

## Context for Development

### Codebase Patterns

Le projet utilise une architecture async avec tokio :

- **Channels mpsc** pour la communication inter-tâches (déjà utilisés dans ChatHub)
- **Arc<RwLock<T>>** pour l'état partagé entre tâches
- **tokio::select!** pour gérer plusieurs sources d'événements concurrentes
- **Pattern Gateway** qui initialise ChatHub et AgentLoop dans src/gateway.rs

**ChatHub** (`src/chat/hub.rs:28-35`):

- Structure avec `inbound_tx/rx`, `outbound_tx/rx`, et `channels: HashMap`
- Méthode `run()` (ligne 211-241) consomme `inbound_rx` mais ne fait que logger
- Déjà utilise `Arc<RwLock<mpsc::Receiver>>` pour le receiver
- Méthode `inbound_sender()` expose le sender pour l'extérieur

**AgentLoop** (`src/agent/agent_loop.rs:62-71`):

- Structure avec `chat_hub: Arc<ChatHub>`, `llm_provider`, `context_builder`, etc.
- Méthode `run()` (ligne 509-541) a un TODO et ne consomme pas de messages
- Méthode `process_message()` (ligne 135-223) existe et fonctionne - construit contexte, appelle LLM, gère tools
- Reçoit `InboundMessage` et retourne `Result<String>` (la réponse)

**Security Whitelist** (`src/utils/security.rs:13-53`):

- `WhitelistChecker` avec `HashSet<i64>` pour les allowed_users
- Méthode `is_allowed()` retourne `false` si whitelist vide (secure-by-default)
- Construit avec `new(allowed_users: Vec<i64>)` et log un warning si vide

**Gateway** (`src/gateway.rs:107-180`):

- Initialise `ChatHub` ligne 108
- Initialise `AgentLoop` ligne 151-158
- Spawn AgentLoop dans une tâche tokio ligne 162-180
- Les deux sont créés mais PAS connectés - c'est le problème à résoudre

### Files to Reference

| File                            | Purpose                                              |
| ------------------------------- | ---------------------------------------------------- |
| src/chat/hub.rs:28-35           | ChatHub struct avec inbound_tx/rx                    |
| src/chat/hub.rs:71-77           | inbound_sender() et outbound_sender()                |
| src/chat/hub.rs:211-241         | Méthode run() qui consomme les messages (à modifier) |
| src/agent/agent_loop.rs:62-71   | AgentLoop struct avec chat_hub field                 |
| src/agent/agent_loop.rs:135-223 | process_message() - existe et fonctionne             |
| src/agent/agent_loop.rs:509-541 | run() - TODO, boucle vide (à implémenter)            |
| src/utils/security.rs:13-53     | WhitelistChecker struct et is_allowed()              |
| src/gateway.rs:107-180          | Initialisation ChatHub et AgentLoop (à connecter)    |
| src/chat/types.rs:8-61          | InboundMessage struct avec channel/chat_id/content   |

### Technical Decisions

- **Option A choisie** : Canal dédié entre ChatHub et AgentLoop pour garder le couplage faible
  - Le ChatHub garde le contrôle de son `inbound_rx` existant
  - Un nouveau canal `agent_tx/rx` sera créé pour forwarder les messages
  - Pattern observé : ChatHub a déjà `channels: HashMap<String, Sender>` pour outbound
- **Channel design** :
  - ChatHub aura un `agent_tx: Option<mpsc::Sender<InboundMessage>>` (Option car pas toujours connecté)
  - AgentLoop recevra `inbound_rx: mpsc::Receiver<InboundMessage>` dans son constructeur
  - Méthode `ChatHub::register_agent_sender(sender)` pour connecter l'AgentLoop
- **Allow-all pattern** (`src/utils/security.rs`) :
  - Wildcard `"*"` dans allowed_users active le mode allow-all
  - Warning log explicite : "WARNING: Allow-all mode enabled - all users allowed!"
  - Le check wildcard est fait AVANT le check empty pour priorité
- **Message flow** :
  1. Telegram → ChatHub.inbound_tx (existant)
  2. ChatHub.run() → recv_inbound() → forward vers agent_tx
  3. AgentLoop.run() → inbound_rx.recv() → process_message()
  4. AgentLoop → chat_hub.reply() → outbound
- **Gestion erreurs** :
  - Si agent_tx est None (pas connecté), messages restent dans inbound_rx (buffer 100)
  - Si agent_tx est full, drop oldest (même logique que send_inbound)
  - Si inbound_rx fermé, AgentLoop log erreur et continue

## Implementation Plan

### Tasks

1. **Modifier src/chat/hub.rs (lignes 28-54, 211-241)**
   - Ajouter champ `agent_tx: Option<mpsc::Sender<InboundMessage>>` au struct ChatHub
   - Initialiser à `None` dans `new()` et `with_capacities()`
   - Ajouter méthode `register_agent_sender(&mut self, sender: mpsc::Sender<InboundMessage>)`
   - Modifier `run()` ligne 216-221 : après le logging debug, forward vers agent_tx si Some()
   - Gérer TrySendError::Full en drop oldest (même pattern que send_inbound ligne 101-110)

2. **Modifier src/agent/agent_loop.rs (lignes 62-115, 509-541)**
   - Ajouter champ `inbound_rx: Option<mpsc::Receiver<InboundMessage>>` au struct AgentLoop
   - Modifier `new()` et `with_model()` pour accepter `Option<mpsc::Receiver<InboundMessage>>`
   - Implémenter logique dans `run()` ligne 522-536 :
     - Remplacer le sleep par `Some(msg) = self.inbound_rx.recv()` dans tokio::select!
     - Appeler `self.process_message(msg).await`
     - Envoyer réponse via `self.chat_hub.reply().await`
     - Gérer erreurs avec tracing::error!()

3. **Modifier src/utils/security.rs (lignes 13-53, 102-179)**
   - Ajouter constante `ALLOW_ALL_WILDCARD: i64 = -1` (valeur spéciale impossible pour vrai user_id)
   - Modifier `new()` ligne 22-34 : détecter wildcard dans allowed_users, log warning spécial
   - Modifier `is_allowed()` ligne 46-53 :
     ```rust
     if self.allowed_users.contains(&ALLOW_ALL_WILDCARD) {
         return true; // Allow-all mode
     }
     ```
   - Ajouter tests unitaires ligne 180+ pour wildcard

4. **Modifier src/gateway.rs (lignes 107-180)**
   - Après ligne 108 (création ChatHub), créer canal : `let (agent_tx, agent_rx) = mpsc::channel(100);`
   - Ligne 109 : appeler `chat_hub.register_agent_sender(agent_tx)` (nécessite mutable, voir note)
   - Ligne 151-158 : passer `Some(agent_rx)` au constructeur AgentLoop
   - Note : ChatHub doit être mutable pour register, ou utiliser RwLock<Option<>>

5. **Créer tests unitaires**
   - src/utils/security.rs : `test_wildcard_allows_all_users()`, `test_wildcard_logs_warning()`
   - src/chat/hub.rs : `test_register_agent_sender()`, `test_message_forwarding_to_agent()`
   - src/agent/agent_loop.rs : `test_run_receives_and_processes_messages()`

6. **Créer tests d'intégration**
   - Créer tests/agent_loop_integration.rs
   - Test `test_end_to_end_message_flow()` :
     - Créer ChatHub, MockChannel, AgentLoop avec MockProvider
     - Envoyer InboundMessage via ChatHub.inbound_sender()
     - Vérifier que la réponse arrive dans outbound
   - Test `test_allow_all_wildcard_integration()`

### Acceptance Criteria

```gherkin
Given un ChatHub configuré avec register_agent_sender()
When un message arrive via inbound_sender()
Then le message est forwardé vers l'AgentLoop via agent_tx

Given un AgentLoop avec inbound_rx configuré
When run() est appelé et un message arrive sur inbound_rx
Then process_message() est appelé et la réponse envoyée via chat_hub.reply()

Given une configuration avec whitelist vide (allowed_users: [])
When is_allowed() est appelé avec n'importe quel user_id
Then retourne false (secure-by-default préservé)

Given une configuration avec allowed_users contenant -1 (wildcard)
When is_allowed() est appelé
Then retourne true ET un warning "Allow-all mode enabled" est loggé

Given le gateway démarré avec ChatHub et AgentLoop
When un message Telegram est reçu
Then il traverse: TelegramChannel → ChatHub → AgentLoop → réponse envoyée

Given le buffer agent_tx est plein (100 messages)
When un nouveau message arrive
Then le plus ancien est drop et le nouveau est ajouté (même logique que inbound)

Given l'AgentLoop reçoit un message
When process_message() échoue (ex: LLM error)
Then l'erreur est loggée mais AgentLoop continue de tourner (graceful degradation)
```

## Additional Context

### Dependencies

Aucune nouvelle dépendance requise. Utilise les crates existantes :

- **tokio** (déjà utilisé) : runtime async, mpsc channels, tokio::select!
- **tracing** (déjà utilisé) : logs structurés avec info!, warn!, error!
- **Arc/RwLock** (stdlib) : partage d'état entre tâches

Note : Le ChatHub utilise déjà `tokio::sync::mpsc` et `tokio::sync::RwLock`, donc pas de changement d'architecture majeur.

### Testing Strategy

**Tests unitaires** (dans les fichiers sources avec `#[cfg(test)]`) :

1. **src/utils/security.rs** (après ligne 179) :

   ```rust
   #[test]
   fn test_wildcard_allows_all_users() {
       let checker = WhitelistChecker::new(vec![ALLOW_ALL_WILDCARD]);
       assert!(checker.is_allowed(123));
       assert!(checker.is_allowed(456));
       assert!(checker.is_allowed(-999)); // Même les IDs invalides
   }
   ```

2. **src/chat/hub.rs** (après ligne 486) :

   ```rust
   #[tokio::test]
   async fn test_register_agent_sender() {
       let mut hub = ChatHub::new();
       let (tx, _rx) = mpsc::channel(10);
       hub.register_agent_sender(tx);
       assert!(hub.agent_tx.is_some());
   }

   #[tokio::test]
   async fn test_message_forwarding_to_agent() {
       // Envoyer message via inbound_sender
       // Vérifier qu'il arrive sur agent_tx
   }
   ```

3. **src/agent/agent_loop.rs** (après ligne 846) :
   ```rust
   #[tokio::test]
   async fn test_run_receives_and_processes_messages() {
       // Créer AgentLoop avec MockProvider
       // Envoyer message sur inbound_rx
       // Vérifier que process_message est appelé
   }
   ```

**Tests d'intégration** (dans `tests/agent_loop_integration.rs`) :

1. **test_end_to_end_message_flow()** :
   - Setup : ChatHub + MockChannel + AgentLoop avec MockProvider
   - Action : Envoyer InboundMessage via hub.inbound_sender()
   - Assert : Réponse reçue sur outbound

2. **test_allow_all_wildcard_integration()** :
   - Setup : Config avec allowed_users: [-1]
   - Action : Vérifier que n'importe quel user_id passe
   - Assert : Warning "Allow-all mode" dans les logs

**Couverture cible** :

- [ ] Happy path : Message reçu → traité → réponse envoyée
- [ ] Erreur : agent_tx full → drop oldest
- [ ] Erreur : inbound_rx fermé → AgentLoop log et continue
- [ ] Edge case : ChatHub sans agent connecté → messages bufferisés
- [ ] Edge case : Allow-all avec vrai user ID -1 (impossible mais testé)
- [ ] Performance : 100 messages/sec sans perte

### Notes

**Implémentation technique détaillée** :

1. **ChatHub ligne 216-221** - La logique actuelle consomme le message mais ne fait que logger. Doit forwarder vers agent_tx si présent.

2. **AgentLoop ligne 509-541** - Structure avec TODO et sleep. Doit recevoir sur inbound_rx et appeler process_message().

3. **Security ligne 46-53** - Logique secure-by-default. Doit vérifier ALLOW_ALL_WILDCARD (-1) avant is_empty().

4. **Gateway ligne 107-180** - Ordre d'initialisation critique. ChatHub → canal → register → AgentLoop.

**Points d'attention** :

- **Mutabilité** : ChatHub dans Arc, besoin de &mut pour register_agent_sender(). Solutions: Arc::get_mut() ou RwLock<Option<>>.
- **Buffer size** : Canal ChatHub-AgentLoop = 100 messages (même capacité que inbound).
- **Graceful degradation** : Si AgentLoop panique, ChatHub bufferise. Gateway redémarre après 5s (déjà implémenté).
- **Tests existants** : Changement de signature de new() doit être rétrocompatible.

**Architecture post-changement** :

```
TelegramChannel
     ↓ (inbound_tx)
ChatHub
     ↓ (agent_tx) ← NOUVEAU
AgentLoop
     ↓ (process_message)
     ↓ (chat_hub.reply)
ChatHub
     ↓ (outbound_tx)
TelegramChannel
```
