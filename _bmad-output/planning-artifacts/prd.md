---
stepsCompleted: [step-01-init, step-02-discovery, step-03-success, step-04-journeys, step-05-domain, step-06-innovation, step-07-project-type, step-08-scoping, step-09-functional, step-10-nonfunctional, step-11-polish, step-12-complete]
inputDocuments:
  - product-brief-miniclaw-2026-02-14.md
  - PLAN_PROJECT.md
  - README.md (picobot reference)
documentCounts:
  briefCount: 1
  researchCount: 0
  brainstormingCount: 0
  projectDocsCount: 2
classification:
  projectType: CLI tool / Agent IA autonome
  domain: Developer tools / IA edge
  complexity: Moyenne
  projectContext: Brownfield
workflowType: 'prd'
---

# Product Requirements Document - miniclaw

**Author:** Matt
**Date:** 2026-02-14

## Success Criteria

### User Success

- **Simplicity d'installation** : Un utilisateur passe de "je ne connais pas le projet" à "mon agent répond" en moins de 10 minutes
- **Zero friction** : `cargo install miniclaw` → `miniclaw onboard` → configuration guidée → fonctionnel
- **Moment "Aha!"** : Premier message Telegram envoyé et réponse reçue en <2 secondes depuis un Raspberry Pi
- **Adoption quotidienne** : 60% des utilisateurs actifs envoient au moins un message par jour après 2 semaines
- **Fierté hardware** : 0% de mini-PC remis au tiroir après 1 mois d'utilisation

### Business Success

- **Visibilité** : 500+ stars GitHub (6 mois), 1000+ (12 mois)
- **Communauté** : 50+ utilisateurs actifs rapportés (6 mois), 100+ (12 mois)
- **Contribution** : 3+ contributeurs externes (6 mois), 10+ (12 mois)

### Technical Success

- **Performance** : Binaire <15MB, RAM <30MB idle, démarrage <100ms
- **Fiabilité** : Uptime perçu >99%, gestion gracieuse des erreurs
- **Qualité** : Coverage tests >80%, documentation 100% features

### Measurable Outcomes

- Taux de complétion onboarding : >90%
- Rétention J7 : >70%, J30 : >50%
- Messages/jour moyen : >3 par utilisateur actif

## Product Scope

### MVP - Minimum Viable Product

**Objectif** : Parité fonctionnelle avec picobot sur Raspberry Pi

| Module | Fonctionnalités |
|--------|----------------|
| **CLI** | version, onboard, agent -m/-M, gateway, memory read/append/write/recent/rank |
| **Chat Hub** | Channels inbound/outbound (tokio mpsc, buffer 100) |
| **Agent Loop** | Loop principal, max 200 itérations, pattern Receive→Context→LLM→Tools→Reply |
| **Context Builder** | Assembly : System + Bootstrap + Memory + Skills + History + Message |
| **11 Tools** | filesystem, exec, web, message, spawn, cron, write_memory, create_skill, list_skills, read_skill, delete_skill |
| **Memory System** | Short-term (VecDeque 100), Long-term (MEMORY.md), Daily notes, Ranker simple |
| **Session Manager** | Persistence JSON par channel:chat_id, max 50 messages FIFO |
| **Cron Scheduler** | In-memory, one-time (FireAt) + recurring (Interval, min 2min) |
| **Telegram Channel** | Long-polling 30s, whitelist allow_from, text messages |
| **Configuration** | JSON config (~/.miniclaw/config.json), env vars override |
| **Workspace** | Structure complète avec SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md |

**Hors MVP** : Providers additionnels, optimisations Rust avancées, Ranker LLM, autres channels

### Growth Features (Post-MVP)

- **Nouveaux channels** : WhatsApp, Discord, Matrix
- **Multi-providers natifs** : Support direct Anthropic, Google, etc. (au-delà d'OpenAI-compatible)
- **Internationalisation** : Support multilingue (i18n) pour l'interface et la documentation
- **Optimisations poussées** : Binaire <8MB, RAM <20MB via LTO, strip, panic=abort

### Vision (Future)

- **Ranker LLM** : Pertinence mémoire améliorée via LLM
- **Marketplace skills** : Skills communautaires et templates
- **Plugin system** : Extensions tierces
- **Dashboard web** : Monitoring et configuration UI
- **Webhook API** : Intégrations externes

## User Journeys

### 1. Thomas - Le Premier Contact Magique (Parcours Principal)

**Contexte** : Thomas, 35 ans, a un Raspberry Pi 3 qui prend la poussière depuis 3 ans. Il a acheté ce Pi pour un projet domotique jamais terminé.

**Scène d'ouverture** : Un samedi matin, Thomas scrolle sur Reddit et tombe sur un post mentionnant miniclaw. Le slogan "Your forgotten hardware, your new AI companion" l'intrigue immédiatement.

**Action montante** :
1. Il visite le repo GitHub, voit les badges "256MB RAM" et "<15MB binary"
2. Pense : "Même mon vieux Pi 3 peut faire ça ?"
3. Ouvre un terminal et tape : `cargo install miniclaw`
4. L'installation se termine en 20 secondes
5. Lance : `miniclaw onboard`
6. Un assistant interactif lui crée automatiquement le workspace dans `~/.miniclaw/`
7. Édite le `config.json` avec sa clé OpenRouter
8. Crée son bot Telegram via @BotFather (guide dans la doc)
9. Ajoute son user ID à la whitelist
10. Lance : `miniclaw gateway`

**Climax** : Il ouvre Telegram, envoie "Hello miniclaw !" et reçoit une réponse en 1.5 secondes depuis SON Raspberry Pi.

**Résolution** : Thomas sourit. Son Pi, oublié depuis 3 ans, vient de lui répondre intelligemment. Il passe les 2 heures suivantes à tester les outils, créer sa première skill météo, et partage son expérience sur r/raspberry_pi.

---

### 2. Sarah - La Configuration Souveraine

**Contexte** : Sarah, 28 ans, travaille dans la tech et est méfiante envers les services cloud. Elle refuse d'envoyer ses conversations sur des serveurs tiers.

**Scène d'ouverture** : Sarah découvre miniclaw via un article tech sur l'IA edge. Elle est intriguée par la promesse "zero database, zero cloud dependency".

**Action montante** :
1. Lit la documentation technique pour comprendre l'architecture
2. Valide que tout reste local (fichiers JSON/markdown uniquement)
3. Configure miniclaw avec Ollama local sur son Raspberry Pi 4
4. Lance l'installation et l'onboarding
5. Vérifie avec `lsof` et `netstat` qu'aucune connexion externe n'est établie (sauf Telegram)
6. Configure Telegram avec une whitelist stricte

**Climax** : Elle envoie son premier message et vérifie dans les logs que l'appel LLM ne sort pas vers OpenAI mais vers `localhost:11434` (Ollama).

**Résolution** : Sarah se détend pour la première fois en utilisant un assistant IA. Ses données restent chez elle. Elle commence à utiliser `write_memory` pour stocker des infos personnelles, sachant qu'elles sont dans des fichiers markdown sur SON disque. Elle recommande miniclaw à ses collègues privacy-conscious.

---

### 3. Marc - L'Optimisation Parfaite

**Contexte** : Marc, 42 ans, est sysadmin. Il déteste le "bloat" et cherche constamment l'efficacité maximale.

**Scène d'ouverture** : Marc découvre miniclaw dans une discussion Hacker News sur les agents IA légers. Il est sceptique : "Encore un agent qui prétend être léger..."

**Action montante** :
1. Télécharge le binaire et vérifie sa taille : 8.2MB (stripé)
2. Lance `time miniclaw version` : 45ms
3. Démarre le gateway et ouvre `htop` dans un autre terminal
4. Observe la consommation RAM : 18MB idle
5. Envoie des messages en rafale pour stresser le système
6. Vérifie que le binaire n'a pas de dépendances externes : `ldd miniclaw`
7. Inspecte le code source pour la qualité Rust (pas de `unsafe`, gestion d'erreurs propre)

**Climax** : Il benchmark miniclaw contre picobot (Go) : démarrage 15% plus rapide, RAM 10% inférieure. Il édite son commentaire HN : "8MB de binaire, 18MB RAM, démarrage instantané. C'est de l'art."

**Résolution** : Marc devient contributeur actif. Il propose des PR d'optimisation, crée une skill de monitoring système, et écrit un blog post "Why Rust beats Go for AI agents on constrained hardware".

---

### 4. Parcours de Résolution - Quand Ça Coince

**Persona** : Julie, 32 ans, curieuse mais pas développeuse. Elle suit un tutoriel YouTube pour installer miniclaw.

**Le Problème** : Après `miniclaw onboard`, elle ne sait pas comment configurer Telegram.

**Scène d'ouverture** : Julie est bloquée. Elle a installé miniclaw mais le `gateway` ne démarre pas ou elle ne reçoit pas de messages.

**Action montante - Le Guide de Dépannage** :
1. Elle lance `miniclaw onboard --help` et voit l'option `--verbose`
2. Relance avec `miniclaw onboard --verbose` et obtient des logs détaillés
3. Le système détecte : "Configuration Telegram incomplète"
4. Un guide étape par étape s'affiche :
   - "Étape 1 : Messagez @BotFather sur Telegram"
   - "Étape 2 : Tapez /newbot et suivez les instructions"
   - "Étape 3 : Copiez le token ici : [input]"
   - "Étape 4 : Quel est votre user ID Telegram ? [guide pour l'obtenir]"
5. Elle copie-colle son token et son user ID
6. Le système valide : "Connexion Telegram testée ✓"

**Climax** : Après configuration, `miniclaw gateway` démarre sans erreur. Elle reçoit "Configuration réussie ! Votre agent est prêt." sur Telegram.

**Résolution** : Julie comprend que même sans être développeuse, elle peut réussir grâce aux guides intégrés. Elle crée sa première skill simple et se sent fière de son agent personnel.

### Journey Requirements Summary

| Capacité | Source |
|----------|--------|
| Installation one-liner (`cargo install`) | Thomas, Marc |
| Onboarding interactif guidé | Thomas, Julie |
| Configuration Telegram assistée | Julie |
| Support multi-providers (Ollama, OpenRouter) | Sarah, Thomas |
| Documentation embedded dans CLI | Julie |
| Logging verbeux pour débogage | Julie |
| Gestion d'erreurs gracieuse | Tous |
| Performance optimale | Marc |

## CLI Tool Specific Requirements

### Project-Type Overview

miniclaw est un outil en ligne de commande hybride : service daemon pour l'agent IA persistant, avec des commandes interactives pour la gestion et la configuration.

### Technical Architecture Considerations

**Architecture des commandes :**
```
miniclaw [command] [subcommand] [flags]

Commands:
  version           # Affiche la version
  onboard           # Initialise la configuration et le workspace
  agent             # Interactions one-shot avec l'agent
    -m, --message   # Message à envoyer
    -M, --model     # Modèle spécifique à utiliser
  gateway           # Démarre le daemon (mode persistant)
  memory            # Gestion de la mémoire
    read            # Lit la mémoire (today|long)
    append          # Ajoute du contenu
    write           # Écrase la mémoire long terme
    recent          # Mémoire des N derniers jours
    rank            # Recherche sémantique
```

**Modes d'exécution :**
- **Mode interactif** : Commandes ponctuelles (`agent`, `memory`)
- **Mode daemon** : Service longue durée (`gateway`) avec Telegram
- **Mode onboarding** : Configuration guidée interactive

### Configuration Strategy

**Hiérarchie de configuration (priorité croissante) :**

1. **Valeurs par défaut** dans le code
2. **Fichier JSON** : `~/.miniclaw/config.json`
   - Configuration structurelle (modèles, timeouts, paths)
   - Créé automatiquement par `onboard`
3. **Variables d'environnement** : Pour les secrets et overrides
   - `MINICLAW_API_KEY` / `OPENROUTER_API_KEY`
   - `TELEGRAM_BOT_TOKEN`
   - `MINICLAW_CONFIG_PATH` (pour custom path)
4. **Flags CLI** : Overrides ponctuels
   - `--config /path/to/config.json`
   - `--model google/gemini-2.5-flash`

**Format de sortie :**
- **MVP** : Texte lisible (parité Picobot)
- **Growth** : Support `--format json` pour intégration script

**Shell Completion :**
- Génération via : `miniclaw completion bash|zsh|fish`
- Installation : `source <(miniclaw completion bash)`

### Implementation Considerations

**Gestion des erreurs :**
- Exit codes standards : 0 (succès), 1 (erreur générique), 2 (mauvais usage)
- Messages d'erreur clairs et actionnables
- Suggestions de correction quand pertinent

**Logging :**
- Mode verbose : `--verbose` ou `-v` (pour débogage)
- Niveaux : ERROR, WARN, INFO, DEBUG
- Sortie stderr pour logs, stdout pour résultats

**Sécurité :**
- Permissions fichier config : 0600 (lecture/écriture owner uniquement)
- Variables d'environnement préférées pour les secrets
- Jamais de logging des tokens ou clés API

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**Approche MVP :** Problem-Solving MVP - Résoudre le problème concret de réutilisation du hardware dormant avec une solution IA complète mais minimaliste.

**Objectif MVP :** Démontrer que "votre vieux hardware peut devenir un compagnon IA" en offrant une expérience fluide de bout en bout sur Raspberry Pi 3/4.

**Ressources nécessaires :**
- 1 développeur Rust expérimenté
- 2-3 mois à temps partiel
- CI/CD via GitHub Actions
- Tests sur Raspberry Pi 3 (hardware cible minimal)

### MVP Feature Set (Phase 1)

**Parcours utilisateurs supportés :**
- Thomas (Maker) - Installation complète, onboarding, premier message
- Julie (Résolution) - Configuration assistée Telegram, guides intégrés
- Marc (Benchmarks) - Performance optimale, métriques visibles
- Sarah (Local) - Support Ollama, fonctionnement offline

**Capacités Must-Have :**

| Module | Fonctionnalités MVP | Justification |
|--------|---------------------|---------------|
| **CLI Core** | version, onboard (interactif), agent -m/-M, gateway | Interface utilisateur complète |
| **Chat Hub** | Channels tokio mpsc (buffer 100) | Architecture core de picobot |
| **Agent Loop** | Max 200 itérations, pattern Receive→Context→LLM→Tools→Reply | Logique agent fonctionnelle |
| **Context Builder** | System + Bootstrap + Memory + Skills + History + Message | Assemblage prompt |
| **11 Tools** | filesystem, exec, web, message, spawn, cron, write_memory, create_skill, list_skills, read_skill, delete_skill | Parité picobot |
| **Memory System** | Short-term (VecDeque 100), Long-term (MEMORY.md), Daily notes, Ranker simple | Persistance données |
| **Session Manager** | JSON persistence, max 50 messages FIFO | Contexte conversation |
| **Cron Scheduler** | FireAt (one-time), Interval (min 2min) | Automatisation |
| **Telegram Channel** | Long-polling 30s, whitelist allow_from | Communication principale |
| **Configuration** | JSON file + env vars override | Flexibilité déploiement |
| **Workspace** | SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md | Personnalisation |
| **Logging** | --verbose flag, niveaux ERROR/WARN/INFO/DEBUG | Debug et monitoring |

**Hors MVP (consciemment dépriorisé) :**
- Formats de sortie JSON
- Shell completion
- Ranker LLM avancé
- Providers autres qu'OpenAI-compatible

### Post-MVP Features

**Phase 2 - Growth (6 mois post-MVP) :**

| Priorité | Feature | Valeur ajoutée |
|----------|---------|----------------|
| **P0** | **i18n Support** | CLI et documentation en FR, ES, DE pour adoption internationale |
| **P1** | **WhatsApp Channel** | Expansion utilisateurs (WhatsApp très répandu) |
| **P2** | **Discord Channel** | Communauté tech, bots serveurs |
| **P3** | **Providers natifs** | Anthropic, Google direct (au-delà OpenAI-compatible) |
| **P4** | **Optimisations Rust** | LTO, strip, panic=abort pour <8MB binaire, <20MB RAM |
| **P5** | **JSON Output** | `--format json` pour intégrations script |

**Phase 3 - Expansion (12+ mois) :**

- **Ranker LLM** : Pertinence mémoire via LLM scoring
- **Marketplace Skills** : Repository communautaire de skills
- **Plugin System** : Extensions tierces via WASM ou DLL
- **Dashboard Web** : Interface web de monitoring et configuration
- **Webhook API** : Intégrations HTTP entrantes/sortantes

### Risk Mitigation Strategy

**Risque Technique : Performance (Critique)**

| Risque | Probabilité | Impact | Mitigation |
|--------|-------------|--------|------------|
| RAM >30MB sur Pi 3 | Moyenne | Critique | Benchmarks continus, profiling régulier, architecture modulaire pour désactiver features gourmandes |
| Binaire >15MB | Faible | Moyen | Optimisations compilation (strip, LTO) en Growth si nécessaire |
| Démarrage >100ms | Faible | Faible | Async loading, lazy initialization |

**Stratégie :**
- Tests de performance dès le début (pas à la fin)
- Profiling avec `cargo flamegraph` et `heaptrack`
- CI avec benchmarks sur chaque PR

**Risque Technique : Complexité Rust (Élevé)**

| Risque | Probabilité | Impact | Mitigation |
|--------|-------------|--------|------------|
| Courbe d'apprentissage async Rust | Moyenne | Moyen | Documentation extensive, patterns éprouvés (tokio), pas d'expérimentations |
| Gestion mémoire complexe | Faible | Critique | Ownership clair, pas de `unsafe`, revues de code strictes |
| Debugging async difficile | Moyenne | Moyen | Logging verbeux, tracing intégré, tests unitaires exhaustifs |

**Stratégie :**
- Architecture simple et directe (pas de over-engineering)
- Tests unitaires >80% coverage avant merge
- Pattern éprouvés (pas de nightly features Rust)
- Revue architecture par pair sur les parties critiques

**Risque Marché : Validation (Moyen)**

| Risque | Probabilité | Impact | Mitigation |
|--------|-------------|--------|------------|
| Peu d'intérêt communautaire | Moyenne | Élevé | Publication précoce (README, demo), engagement Reddit/HN, collecte feedback rapide |
| Picobot suffisant pour users | Faible | Moyen | Différenciation claire (Rust + performance) |

**Stratégie :**
- Release "alpha" dès que core fonctionnel (sans tous les outils)
- Communauté avant perfection (ship early, iterate)
- Focus makers/Raspberry Pi (niche identifiable)

## Functional Requirements

### CLI Interface & Commandes

- **FR1** : L'utilisateur peut afficher la version de miniclaw via la commande `version`
- **FR2** : L'utilisateur peut initialiser la configuration et le workspace via la commande `onboard`
- **FR3** : L'utilisateur peut envoyer une requête unique à l'agent via `agent -m "message"`
- **FR4** : L'utilisateur peut spécifier un modèle LLM particulier via `agent -M model -m "message"`
- **FR5** : L'utilisateur peut démarrer le mode daemon via la commande `gateway`
- **FR6** : L'utilisateur peut lire la mémoire (today ou long) via `memory read`
- **FR7** : L'utilisateur peut ajouter du contenu à la mémoire via `memory append`
- **FR8** : L'utilisateur peut écraser la mémoire long terme via `memory write`
- **FR9** : L'utilisateur peut consulter les mémoires des N derniers jours via `memory recent --days N`
- **FR10** : L'utilisateur peut rechercher des mémoires pertinentes via `memory rank -q "query"`
- **FR11** : L'utilisateur peut activer le mode verbeux pour le débogage via `--verbose`

### Agent Conversationnel

- **FR12** : L'agent peut recevoir des messages via le Chat Hub (channels tokio mpsc)
- **FR13** : L'agent peut traiter jusqu'à 200 itérations d'outils par message
- **FR14** : L'agent peut assembler un contexte complet (System + Bootstrap + Memory + Skills + History + Message)
- **FR15** : L'agent peut appeler des outils et recevoir leurs résultats
- **FR16** : L'agent peut répondre via le canal de communication approprié
- **FR17** : L'agent peut maintenir une session de conversation persistante (max 50 messages FIFO)

### Système de Mémoire

- **FR18** : Le système peut stocker la mémoire à court terme (VecDeque, limit 100 entrées)
- **FR19** : Le système peut persister la mémoire long terme dans un fichier MEMORY.md
- **FR20** : Le système peut créer des notes quotidiennes automatiques (YYYY-MM-DD.md)
- **FR21** : Le système peut récupérer les mémoires les plus récentes
- **FR22** : Le système peut classer les mémoires par pertinence (ranker simple par mots-clés)
- **FR23** : Le système peut écrire de nouvelles mémoires via l'outil write_memory

### Outils & Capacités

- **FR24** : Le système peut lire, écrire et lister des fichiers via l'outil filesystem
- **FR25** : Le système peut exécuter des commandes shell via l'outil exec (avec restrictions de sécurité)
- **FR26** : Le système peut récupérer du contenu web via l'outil web
- **FR27** : Le système peut envoyer des messages via le Chat Hub via l'outil message
- **FR28** : Le système peut lancer des tâches en arrière-plan via l'outil spawn
- **FR29** : Le système peut planifier des tâches ponctuelles (FireAt) via l'outil cron
- **FR30** : Le système peut planifier des tâches récurrentes (Interval, min 2min) via l'outil cron
- **FR31** : Le système peut créer des packages de skills via l'outil create_skill
- **FR32** : Le système peut lister les skills disponibles via l'outil list_skills
- **FR33** : Le système peut lire le contenu d'un skill via l'outil read_skill
- **FR34** : Le système peut supprimer un skill via l'outil delete_skill

### Gestion des Canaux

- **FR35** : Le système peut recevoir des messages via Telegram (long-polling, timeout 30s)
- **FR36** : Le système peut filtrer les messages Telegram par whitelist (allow_from)
- **FR37** : Le système peut envoyer des réponses via Telegram
- **FR38** : Le système peut traiter uniquement les messages texte via Telegram (MVP)

### Configuration & Workspace

- **FR39** : Le système peut charger la configuration depuis un fichier JSON (~/.miniclaw/config.json)
- **FR40** : Le système peut surcharger la configuration via des variables d'environnement
- **FR41** : Le système peut accepter des overrides via des flags CLI
- **FR42** : Le système peut créer automatiquement la structure de workspace (SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md)
- **FR43** : Le système peut charger les skills depuis le dossier workspace/skills/
- **FR44** : Le système peut persister les sessions dans workspace/sessions/

### Logging & Monitoring

- **FR45** : Le système peut logger des messages aux niveaux ERROR, WARN, INFO, DEBUG
- **FR46** : Le système peut sortir les logs sur stderr et les résultats sur stdout
- **FR47** : Le système peut afficher des métriques de performance (utilisation RAM, temps de réponse)

## Non-Functional Requirements

### Performance

- **NFR-P1** : Le binaire compilé ne doit pas dépasser 15MB (mesuré avec `strip`)
- **NFR-P2** : La consommation RAM au repos ne doit pas dépasser 30MB sur Raspberry Pi 3
- **NFR-P3** : Le temps de démarrage (cold start) doit être inférieur à 100ms
- **NFR-P4** : Le temps de réponse aux messages Telegram doit être inférieur à 2 secondes (95e percentile)
- **NFR-P5** : Le système doit supporter jusqu'à 100 messages en attente dans le buffer du Chat Hub sans perte

### Sécurité

- **NFR-S1** : Les clés API et tokens doivent être stockés uniquement dans des variables d'environnement ou fichier avec permissions 0600
- **NFR-S2** : Aucun secret ne doit apparaître dans les logs (même en mode verbose)
- **NFR-S3** : Les chemins de fichiers doivent être validés et résolus via `canonicalize()` pour prévenir les attaques path traversal
- **NFR-S4** : L'outil exec doit refuser d'exécuter les commandes blacklisted (rm, sudo, dd, mkfs, shutdown, reboot, etc.)
- **NFR-S5** : Seuls les utilisateurs présents dans la whitelist Telegram peuvent interagir avec l'agent
- **NFR-S6** : Les communications avec les APIs LLM doivent utiliser HTTPS/TLS 1.2 minimum

### Fiabilité

- **NFR-R1** : Le système doit redémarrer automatiquement en cas de crash (avec systemd ou docker --restart)
- **NFR-R2** : Les erreurs doivent être loggées avec niveau ERROR et message explicite
- **NFR-R3** : Le système doit jamais panic sur une entrée utilisateur invalide
- **NFR-R4** : Les sessions doivent être persistées automatiquement toutes les 30 secondes
- **NFR-R5** : Le système doit supporter une interruption gracieuse (SIGTERM) avec flush des données

### Compatibilité

- **NFR-C1** : Le système doit fonctionner sur Linux ARM64 (Raspberry Pi 3/4)
- **NFR-C2** : Le système doit fonctionner sur Linux AMD64 (VPS, mini-PC x86)
- **NFR-C3** : Le binaire ne doit avoir aucune dépendance runtime externe (libc standard uniquement)
- **NFR-C4** : La configuration doit être compatible avec Docker et Docker Compose
- **NFR-C5** : Le système doit fonctionner sur Windows x86-64
