---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments:
  - docs/PLAN_PROJECT.md
  - docs/references/picobot/README.md
  - docs/references/picobot/HOW_TO_START.md
  - docs/references/picobot/CONFIG.md
  - docs/references/picobot/internal/agent/context.go
  - docs/references/picobot/internal/chat/chat.go
  - docs/references/picobot/internal/config/schema.go
date: 2026-02-14
author: Matt
---

# Product Brief: miniclaw

<!-- Content will be appended sequentially through collaborative workflow steps -->

## Documents de référence chargés

Ce brief produit est basé sur les documents suivants :

### Plan de projet initial
- **PLAN_PROJECT.md** - Spécifications techniques détaillées pour MiniClaw (agent IA en Rust)

### Référence picobot (portage Go → Rust)
- **README.md** - Vue d'ensemble de picobot, architecture et fonctionnalités
- **HOW_TO_START.md** - Guide de démarrage et configuration
- **CONFIG.md** - Référence complète de configuration
- **internal/agent/context.go** - Implémentation du ContextBuilder
- **internal/chat/chat.go** - Hub de messages inbound/outbound
- **internal/config/schema.go** - Schéma de configuration

---

## Executive Summary

MiniClaw est un agent IA autonome ultra-léger écrit en Rust, conçu pour fonctionner sur des mini-PC et Raspberry Pi avec des ressources limitées (à partir de 256MB RAM). Face aux solutions existantes trop gourmandes (OpenClaw en Python) ou trop complexes (IronClaw avec PostgreSQL), MiniClaw offre un binaire unique, rapide et simple à déployer, permettant à chacun d'héberger son propre agent IA localement sans compromis sur les performances ou la simplicité.

**Promesse fondamentale :** *"Your forgotten hardware, your new AI companion"*

MiniClaw redonne vie aux vieux mini-PC oubliés dans les tiroirs en les transformant en assistants IA personnels, toujours disponibles H24, sans coût énergétique significatif ni infrastructure complexe.

---

## Core Vision

### Problem Statement

Les frameworks d'agents IA actuels souffrent d'un problème majeur de lourdeur et de complexité :
- **OpenClaw** (Python) : Ultra-complet mais impossible à faire fonctionner sur des mini-PC anciens ou avec peu de RAM
- **IronClaw** (Rust) : Prometteur mais nécessite une infrastructure complexe (PostgreSQL) difficile à mettre en place
- **Autres solutions** : Souvent écrites en Python ou TypeScript, imposant des runtimes lourds et des dépendances nombreuses

Les makers et développeurs possédant des Raspberry Pi anciens (1GB RAM) ou soucieux de préserver les ressources de leur système se retrouvent exclus de l'expérience agent IA autonome, contraints de recourir à des solutions cloud coûteuses ou complexes.

### Problem Impact

- **Exclusion technologique** : Les utilisateurs avec matériel ancien ne peuvent pas profiter de l'IA autonome
- **Hardware dormant** : Des millions de mini-PC (Raspberry Pi, Banana Pi, etc.) finissent dans les tiroirs faute d'utilité
- **Coût et dépendance** : Obligation de passer par des services cloud avec problématiques de privacy et coûts récurrents
- **Friction de déploiement** : Même les solutions "légères" existantes comme picobot (Go) pourraient être encore plus optimisées
- **Complexité inutile** : Besoin d'une base de données PostgreSQL pour un agent personnel est overkill pour la plupart des cas d'usage

### Why Existing Solutions Fall Short

| Solution         | Échec principal                                                            |
| ---------------- | -------------------------------------------------------------------------- |
| **OpenClaw**         | Trop gourmand en ressources (Python + dépendances), impossible sur mini-PC |
| **IronClaw**         | Complexité d'installation (PostgreSQL requis), barrière à l'entrée élevée  |
| **Nanobot/Nanoclaw** | Souvent en Python/TypeScript avec overhead runtime conséquent              |
| **Picobot (Go)**     | Bonne solution mais Rust permet d'aller encore plus loin en optimisation   |

Les solutions existantes négligent le segment des utilisateurs qui veulent simplement "un agent qui fonctionne" sans infrastructure complexe, sur leur matériel existant.

### Proposed Solution

MiniClaw est un agent IA autonome écrit en Rust offrant :

- **Binaire ultra-léger** : ~5-10MB (vs ~12MB pour picobot, 200MB+ pour solutions Python)
- **RAM minimale** : Fonctionnement sur 256MB RAM (vs 1GB+ requis ailleurs)
- **Zéro dépendance runtime** : Single binary, pas de Python, Node ou Docker obligatoire
- **Démarrage instantané** : <100ms grâce à Rust et optimisation compilation
- **Multi-providers** : Support natif OpenAI, OpenRouter, Ollama local, etc.
- **Configuration guidée** : Commande `miniclaw onboard` pour setup pas à pas
- **Sans base de données** : Stockage fichier simple (JSON/markdown) suffisant
- **LLM local possible** : Support Ollama pour souveraineté data complète

Fonctionnalités identiques à picobot : Chat Hub, Agent Loop, 11 outils (filesystem, exec, web, cron, memory, skills...), Telegram integration, mémoire persistante.

### Key Differentiators

1. **Performance maximale** : Rust permet d'aller au-delà des performances Go de picobot (sécurité mémoire, zéro garbage collection, optimisation compilation LTO)
2. **Zero Database** : Contrairement à IronClaw, pas de PostgreSQL requis - stockage simple fichiers
3. **Multi-providers natifs** : Support flexible pour différents providers LLM sans configuration complexe
4. **Onboarding frictionless** : Commande `miniclaw onboard` créant workspace + config automatiquement
5. **Always-on H24** : Consommation énergétique négligeable permettant de laisser l'agent tourner en permanence
6. **Timing parfait** : Adoption croissante de l'AI edge + tarifs providers devenus accessibles + maturité écosystème Rust (tokio, clap, serde)

**Value Proposition** : "Your forgotten hardware, your new AI companion" - Redonnez vie à votre vieux mini-PC en le transformant en assistant personnel toujours disponible, sans coût additionnel.

**Modèle** : Open source Apache-2.0, gratuit, donations possibles plus tard.

---

## Target Users

### Primary Users

#### Persona 1: Thomas, le Maker nostalgique

**Profil :** 35 ans, développeur amateur, possède un Raspberry Pi 3 dans un tiroir depuis 3 ans

**Contexte :** A acheté son Pi pour un projet jamais terminé. Le hardware fonctionne mais ne sert à rien. Intéressé par l'IA mais les solutions existantes sont trop lourdes pour son matériel.

**Motivations :**
- Redonner vie à son hardware dormant
- Expérimenter l'IA sans investir dans du nouveau matériel
- Apprendre et bricoler avec un projet concret

**Job-to-be-done :** Transformer son Pi oublié en assistant personnel utile et fonctionnel

**Frustrations actuelles :**
- OpenClaw impossible à installer sur son Pi (trop gourmand)
- IronClaw trop complexe avec sa base PostgreSQL
- Picobot bien mais pourrait être encore plus léger

**Moment "Aha!" :** Quand il envoie son premier message Telegram et reçoit une réponse intelligente en moins d'une seconde depuis son "vieux" Pi.

**Quote :** "Je savais pas quoi faire de ce Pi, maintenant c'est mon assistant perso qui tourne H24 !"

---

#### Persona 2: Sarah, la souveraine de ses données

**Profil :** 28 ans, travaille dans la tech, méfiante envers les services cloud grand public

**Contexte :** Consciente des enjeux de privacy. Veut utiliser l'IA mais refuse d'envoyer ses données sur des serveurs tiers. Cherche une solution self-hosted simple.

**Motivations :**
- Souveraineté totale sur ses données
- Pas de dépendance aux APIs cloud
- Solution locale et contrôlée
- Coût maîtrisé (juste électricité du Pi)

**Job-to-be-done :** Avoir un assistant IA personnel qui vit chez elle, pas sur des serveurs distants

**Frustrations actuelles :**
- ChatGPT et consorts stockent toutes les conversations
- Solutions self-hosted existantes trop complexes à maintenir
- Besoin d'infrastructure lourde (Docker, DB, etc.)

**Moment "Aha!" :** Quand elle configure Ollama en local et réalise que tout fonctionne sans aucune connexion API externe.

**Quote :** "Mon assistant IA vit chez moi, mes données restent chez moi. Point final."

---

#### Persona 3: Marc, l'optimisateur compulsif

**Profil :** 42 ans, sysadmin, passionné par l'efficacité et le minimalisme technique

**Contexte :** Adore trouver la solution la plus élégante et légère possible. Déteste le "bloat" et les logiciels gourmands. Toujours à la recherche de l'optimisation parfaite.

**Motivations :**
- Démontrer que l'efficacité prime sur la complexité
- Trouver la solution la plus légère possible
- Benchmarker et optimiser
- Partager ses découvertes avec la communauté

**Job-to-be-done :** Faire tourner un agent IA complet en moins de 30MB RAM et prouver que c'est possible

**Frustrations actuelles :**
- La plupart des agents IA sont des "usines à gaz"
- Python = overhead inacceptable pour lui
- Même Go n'est pas assez optimisé à son goût

**Moment "Aha!" :** Quand il vérifie avec `htop` et voit que MiniClaw consomme 18MB RAM et démarre en 50ms.

**Quote :** "8MB de binaire, 18MB RAM, démarrage instantané. C'est de l'art."

---

### User Journey

#### Phase 1: Découverte
**Touchpoint :** GitHub, Hacker News, Reddit r/selfhosted, forums Raspberry Pi

**Action :** Thomas cherche "lightweight AI agent raspberry pi" et tombe sur MiniClaw. Le README promet "Your forgotten hardware, your new AI companion". Il est intrigué.

**Émotion :** Curiosité + scepticisme ("Encore un projet qui promet tout...")

---

#### Phase 2: Onboarding
**Touchpoint :** Terminal après installation

**Action :** `miniclaw onboard` créé automatiquement :
- `~/.miniclaw/config.json`
- Workspace avec SOUL.md, AGENTS.md, USER.md, TOOLS.md
- Structure mémoire et skills

**Émotion :** Surprise agréable ("C'est tout ? Ça a pris 10 secondes !")

---

#### Phase 3: Configuration
**Touchpoint :** Édition config.json + BotFather Telegram

**Action :** 
1. Configure sa clé API OpenRouter
2. Crée son bot Telegram via @BotFather
3. Ajoute son user ID à la whitelist
4. Lance `miniclaw gateway`

**Émotion :** Excitation ("Ça va vraiment marcher ?")

---

#### Phase 4: Première Valeur
**Touchpoint :** Application Telegram

**Action :** Envoie "Hello" à son bot. Réponse en <2 secondes depuis son Raspberry Pi.

**Émotion :** **WOW** ("Mon Pi de 2016 vient de me répondre intelligemment !")

---

#### Phase 5: Adoption Quotidienne
**Touchpoint :** Utilisation régulière

**Action :**
- Discussions via Telegram tout au long de la journée
- Création de skills pour automatiser des tâches
- Utilisation de write_memory pour persister des infos importantes
- Heartbeat pour tâches périodiques

**Émotion :** Satisfaction + fierté ("J'ai mon assistant perso qui tourne sur mon vieux matos")

---

#### Phase 6: Advocacy
**Touchpoint :** Communauté

**Action :** Partage son setup sur Reddit, GitHub issues, forums. Recommande à d'autres makers.

**Émotion :** Enthousiasme évangéliste ("Faut absolument que j'en parle autour de moi !")

---

## Success Metrics

### User Success Metrics

| Métrique | Cible | Description |
|----------|-------|-------------|
| **Premier "Wow"** | 80% | Des utilisateurs réussissent leur premier message Telegram en <5 min après `onboard` |
| **Adoption quotidienne** | 60% | Des utilisateurs actifs envoient au moins un message par jour après 2 semaines |
| **Resurrection hardware** | 0% | De mini-PC remis dans le tiroir après 1 mois d'utilisation |
| **Feature adoption** | 40% | Des utilisateurs ont créé au moins 1 skill personnalisé après 1 mois |

**User Success Definition :**
Un utilisateur réussit quand il transforme son mini-PC dormant en compagnon IA quotidien, sans jamais avoir à se demander si son agent est "disponible" avant de lui demander une action.

---

### Business Objectives (Open Source)

| Objectif | Cible 6 mois | Cible 12 mois |
|----------|--------------|---------------|
| **Stars GitHub** | 500+ | 1000+ |
| **Utilisateurs actifs** | 50+ rapportés | 100+ rapportés |
| **Contributeurs externes** | 3+ | 10+ |
| **Satisfaction** | >4/5 | >4.5/5 |

**Stratégie Open Source :**
- Licence Apache-2.0 pour adoption maximale
- Gratuit avec possibilité de donations futures
- Communauté prioritaire sur monetization

---

### Key Performance Indicators (KPIs)

#### KPIs Techniques

| KPI | Cible | Méthode de mesure |
|-----|-------|-------------------|
| **Taux complétion onboarding** | >90% | Analytics installation vs utilisation |
| **Consommation RAM** | <30MB | Benchmarks sur Raspberry Pi 3/4 |
| **Taille binaire** | <15MB | `cargo build --release` + strip |
| **Démarrage** | <100ms | Mesure temps cold start |
| **Uptime perçu** | >99% | Feedback utilisateur always-on |

#### KPIs d'Engagement

| KPI | Cible | Description |
|-----|-------|-------------|
| **Messages/jour** | >3 | Moyenne par utilisateur actif |
| **Rétention J7** | >70% | Utilisateurs actifs après 7 jours |
| **Rétention J30** | >50% | Utilisateurs actifs après 30 jours |
| **Création skills** | >0.5/mois | Skills créés par utilisateur actif |

#### KPIs Qualité

| KPI | Cible | Description |
|-----|-------|-------------|
| **Issues résolues** | >90% | Taux de résolution sous 30 jours |
| **Time to response** | <48h | Premier retour sur issues GitHub |
| **Documentation** | Complète | Coverage 100% des features |
| **Tests** | >80% | Coverage code source |

---

### Strategic Alignment

**Comment ces métriques connectent la vision au succès :**

1. **Vision** : "Your forgotten hardware, your new AI companion"
   → **Métrique** : 0% hardware retourné au tiroir

2. **Problème** : Solutions existantes trop lourdes/complexes
   → **Métrique** : <30MB RAM, onboarding >90% success

3. **Différenciation** : Performance Rust + Zero DB + Always-on
   → **Métrique** : <100ms démarrage, uptime >99%

4. **Modèle** : Open source communautaire
   → **Métrique** : 1000+ stars, 10+ contributeurs

**Alertes early-warning :**
- Si rétention J7 <50% : Problème onboarding ou première valeur
- Si RAM >30MB : Régression performance critique
- Si stars <100 en 3 mois : Problème visibilité marketing

---

## MVP Scope

### Core Features (Parité Picobot)

| Module | Fonctionnalités MVP |
|--------|---------------------|
| **CLI** | `version`, `onboard`, `agent -m`, `agent -M`, `gateway`, `memory read/append/write/recent/rank` |
| **Chat Hub** | Inbound/Outbound channels (tokio mpsc, buffer 100) |
| **Agent Loop** | Loop principal avec max 200 itérations, pattern Receive→Context→LLM→Tools→Reply |
| **Context Builder** | Assembly : System + Bootstrap files + Memory + Skills + History + Current message |
| **11 Tools** | filesystem, exec, web, message, spawn, cron, write_memory, create_skill, list_skills, read_skill, delete_skill |
| **Memory System** | Short-term (VecDeque 100), Long-term (MEMORY.md), Daily notes (YYYY-MM-DD.md), Ranker simple |
| **Session Manager** | Persistence JSON par channel:chat_id, max 50 messages FIFO |
| **Cron Scheduler** | In-memory, one-time (FireAt) + recurring (Interval, min 2min) |
| **Telegram Channel** | Long-polling 30s, whitelist allow_from, text messages |
| **Configuration** | JSON config (~/.miniclaw/config.json), env vars override |
| **Workspace** | Structure complète avec SOUL.md, AGENTS.md, USER.md, TOOLS.md, HEARTBEAT.md, memory/, sessions/, skills/ |

### Out of Scope for MVP

- Providers additionnels au-delà d'OpenAI-compatible (Anthropic direct, etc.)
- Optimisations avancées Rust (LTO, strip) visant <5MB
- Ranker LLM (vs ranker simple par mots-clés)
- Autres channels (Discord, WhatsApp)
- UI web de monitoring
- Métriques d'usage intégrées
- Plugins système externe

### MVP Success Criteria

| Critère | Cible | Validation |
|---------|-------|------------|
| **Parité fonctionnelle** | 100% | Toutes les commandes CLI de picobot fonctionnent identiquement |
| **Taille binaire** | <15MB | Même seuil que picobot |
| **Consommation RAM** | <30MB | Sur Raspberry Pi 3 idle |
| **Outils opérationnels** | 11/11 | Tous les outils fonctionnels |
| **Validation utilisateur** | Réussite | Test comparatif picobot vs miniclaw |

### Future Vision

**Phase 2 - Optimisation Poussée :**
- Binaire <8MB via optimisations Rust avancées (LTO, strip, panic=abort)
- RAM <20MB idle sur Raspberry Pi
- Ranker LLM pour pertinence mémoire améliorée

**Phase 3 - Expansion :**
- Support natif multi-providers (Anthropic, Google, etc.)
- Nouveaux channels (Discord, WhatsApp, Matrix)
- Webhook API pour intégrations externes

**Phase 4 - Écosystème :**
- Marketplace skills communautaire
- Templates skills par défaut
- Plugin system pour extensions tierces
- Dashboard web de monitoring
