# Pylos — AI Gateway & MCP Proxy

## Présentation générale

Pylos est un **AI Gateway** haute performance, réécrit en Rust, qui sert de point d'accès unifié à **20+ fournisseurs de modèles de langage (LLM)**. Il est compatible avec les SDK OpenAI : il suffit de pointer son `base_url` vers Pylos pour bénéficier de l'ensemble des fonctionnalités sans changer une ligne de code.

---

## 1. Compatibilité OpenAI

### Endpoints supportés
- `POST /v1/chat/completions` — Chat completions (streaming et non-streaming)
- `POST /v1/completions` — Completions texte legacy (conversion automatique vers chat)
- `POST /v1/embeddings` — Embeddings
- `POST /v1/images/generations` — Génération d'images
- `GET /v1/models` — Listing des modèles disponibles

### Compatibilité SDK
- **Drop-in replacement** pour les SDK OpenAI (Python, Node.js, etc.)
- Configuration minimale : changer `base_url` et utiliser une clé virtuelle Pylos (`sk-pylos-*`)

---

## 2. Gestion des fournisseurs d'IA

### 6 adaptateurs natifs
| Fournisseur | Protocole |
|---|---|
| **OpenAI** & compatibles (Groq, Ollama, OpenRouter, vLLM, Mistral, Cerebras, Perplexity, Fireworks, xAI, Nebius, DeepSeek, Lemonade, Vertex) | API OpenAI |
| **Anthropic** | API Claude |
| **AWS Bedrock** | Converse / ConverseStream API |
| **Azure OpenAI** | Azure OpenAI Service |
| **Google Gemini** | Gemini API |
| **Cohere** | Cohere API v2 |

### Auto-détection des fournisseurs
- Détection automatique du fournisseur à partir du nom du modèle
- Exemples : `gpt-*` → OpenAI, `claude-*` → Anthropic, `gemini-*` → Gemini

### Gestion des clés API
- **Multi-clés pondérées** : plusieurs clés API par fournisseur avec poids configurables
- **Load balancing** : algorithme A-Res (weighted random selection)
- **Circuit breaker** : après 5 échecs consécutifs, un fournisseur est désactivé 30 secondes
- **Retry avec backoff exponentiel** et jitter

### Gestion des fournisseurs en runtime
- CRUD complet des fournisseurs via l'API de management ou l'interface web
- Configuration réseau (timeouts, endpoints personnalisés)
- Statut de connectivité visible dans l'interface

---

## 3. Routage intelligent

### Routage par modèle
- Association automatique modèle → fournisseur
- Support des **règles CEL (Common Expression Language)** pour le routage avancé
- Cibles pondérées pour le routage A/B

### Fallback multi-fournisseur
- Si un fournisseur échoue, bascule automatique vers le suivant
- Ordonnancement : fournisseurs supportant le modèle d'abord, puis les autres
- **Model mapping** : traduction des noms de modèles entre fournisseurs (ex: `gemini-2.5-flash` → `deepseek-v4-flash`)

### Streaming
- SSE (Server-Sent Events) avec métriques token-level
- **Time-to-First-Token (TTFT)** et **tokens-per-seconde** trackés
- Support des **tool calls en streaming**
- Support du **reasoning content** (DeepSeek R1, OpenAI o1/o3)

---

## 4. Sécurité et gouvernance

### Clés API virtuelles
- Format `sk-pylos-*`
- **ACL par fournisseur et par modèle** : contrôle granulaire de ce que chaque clé peut utiliser
- Poids (weight) par fournisseur pour la répartition
- Clés avec **date d'expiration**
- Activation/désactivation sans suppression
- Registry in-memory pour lookup ultra-rapide

### Rate limiting
- Limites **RPM** (requêtes par minute) et **TPM** (tokens par minute)
- Appliqué par clé virtuelle
- Persistance SQLite ou PostgreSQL
- Atomic check-and-increment (pas de race conditions)

### Budgets
- Limites budgétaires en USD par clé virtuelle
- Périodes de reset configurables : 30s, 5min, 1h, 1j, 1sem, 1mois, 1an
- Enforcement via pre-hook plugin

### Authentification administrateur
- Protection de toutes les routes de management par clé admin (`PYLOS_ADMIN_KEY`)
- Comparaison en temps constant (constant-time)

### OIDC / JWT
- Validation des tokens JWT (RS256 et HS256)
- Support de n'importe quel fournisseur OIDC

---

## 5. Plugins (architecture Pre/Post hooks)

| Plugin | Fonction |
|---|---|
| **Guardrails** | Masquage PII (emails, téléphones, CB), blocage de mots-clés, détection d'injections prompt |
| **StructuredOutput** | Validation et correction automatique des réponses JSON (`json_object`, `json_schema`) |
| **SemanticCache** | Cache sémantique via Qdrant : réponses mises en cache basées sur la similarité |
| **RAG / Retrieval** | Interception de modèles spécifiques pour injecter du contexte depuis Qdrant |
| **Budget** | Enforcement des limites budgétaires USD |
| **RateLimit** | Enforcement des limites RPM/TPM |
| **Batching** | Accumulation dynamique des requêtes concurrentes pour un même modèle |
| **PrefixCache** | Cache de préfixes pour les tokens prompt |
| **PromptRegistry** | Registre de templates de system prompts |
| **OpenTelemetry** | Attribution de span attributes pour les appels LLM |

Ordre d'exécution : **Pre-hooks** (avant l'appel LLM) → **InferenceOrchestrator** → **Post-hooks** (après la réponse)

---

## 6. Observabilité

### Métriques Prometheus
Endpoint `/metrics` exposant :
- Compteurs de requêtes (par fournisseur, modèle, statut)
- Histogrammes de latence
- Compteurs de tokens (prompt, completion)
- Requêtes en cours (in-flight)
- Time-to-First-Token (TTFT)
- Tokens par seconde

### Tracing distribué (OpenTelemetry)
- Export OTLP HTTP
- Conventions sémantiques `gen_ai.system` par fournisseur

### Logging structuré
- Requête/réponse complète : fournisseur, modèle, latence, tokens, coût USD, statut, clé virtuelle
- Stockage : **SQLite** (WAL mode) ou **PostgreSQL**
- API de requêtage avec filtres, statistiques, histogrammes temporels

### Dashboards Grafana
- Dashboard pré-construit avec provisioning automatique
- Stack complète Docker : Pylos + UI + Prometheus + Grafana

---

## 7. Administration (Interface React)

### Pages de l'interface

| Page | Fonctionnalités |
|---|---|
| **Dashboard** | KPIs (requêtes, succès, latence, tokens, coût), graphiques d'activité, période 1h/6h/24h/7d/30d, auto-refresh 30s |
| **Playground** | Chat interactif, sélecteur de modèle groupé par fournisseur, comparaison A/B, métriques temps réel, export |
| **Logs** | Journal paginé avec filtres (période, fournisseur, statut, clé, modèle), modal détaillé, auto-refresh 10s |
| **Analytics** | Analyses par fournisseur : volume, heatmap latence, coût comparé, économies estimées vs GPT-4o |
| **Providers** | CRUD fournisseurs, clés multiples pondérées, test de connectivité |
| **Virtual Keys** | CRUD clés, ACL fournisseurs/modèles, budget, rate limits, expiration |
| **Model Catalog** | Registre des modèles, prix, fenêtre de contexte, capacités (vision, tools, streaming) |
| **Guardrails** | Activation/désactivation des guardrails, configuration PII, mots-clés |
| **Budgets & Billing** | Barres d'utilisation budgétaire par clé, seuils colorés (vert/jaune/rouge) |
| **Organizations** | Gestion multi-tenant des organisations |
| **Teams** | Gestion des équipes au sein des organisations |
| **Internal Users** | Utilisateurs avec rôles et appartenances |
| **Access Groups** | Groupes d'accès avec permissions modèles/fournisseurs |
| **Policies** | Politiques configurables (JSON) |
| **Tool Policies** | Politiques d'accès par outil MCP |
| **Search Tools** | Configuration des outils de recherche MCP |
| **Vector Stores** | Configuration des stores vectoriels MCP |

---

## 8. Gestion des accès (RBAC multi-tenant)

### Hiérarchie
```
Organizations → Teams → Users → Access Groups → Policies
```

### Fonctionnalités
- **Multi-tenant** : organisations isolées avec leurs équipes et utilisateurs
- **Groupes d'accès** : permissions sur des modèles et fournisseurs spécifiques
- **Tool policies** : contrôle granulaire des outils MCP (modèles autorisés, rate limits par outil)
- **Politiques JSON** : flexibilité totale pour des règles custom

---

## 9. MCP (Model Context Protocol)

- **Proxy MCP** : interface unifiée pour les outils et stores vectoriels
- **Search Tools** : interface de configuration des outils de recherche
- **Vector Stores** : interface de configuration des stores vectoriels
- **Tool Policies** : gestion des politiques d'accès par outil avec API CRUD

---

## 10. Persistance des données

### Bases de données (SQLite)
- `pylos-logs.db` — Logs de requêtes
- `pylos-catalog.db` — Catalogue de modèles
- `pylos-budget.db` — Budgets
- `pylos-ratelimit.db` — Rate limits
- `pylos-virtualkeys.db` — Clés virtuelles
- `pylos-prompts.db` — Prompts templates
- `pylos-config.db` — Configuration

### PostgreSQL
- Base unique remplaçant tous les stores SQLite
- Configurable via `database_url`

### Vector store
- **Qdrant** pour le cache sémantique et le RAG

---

## 11. Configuration

- Fichier unique **`pylos.json`** avec schéma JSON
- Références à des variables d'environnement (`env.VAR_NAME`)
- **Hot-reload** : rechargement à chaud via `POST /config/reload` ou bouton UI
- **Versioning** : format v2 avec sémantique deny-all par défaut
- **SHA-256 hashing** pour détection de changement
- CRUD runtime des providers et clés virtuelles

---

## 12. Déploiement

### Docker
- Image multi-arch (amd64 + arm64)
- Build multi-stage cross-compilé
- Stack complète : `docker-compose up`

### Kubernetes
- **Helm chart** complet dans `helm/pylos/`
- Compatible ArgoCD / GitOps

### CI/CD
- GitHub Actions : lint, test, build, push GHCR
- **Promotion dev→prod** : endpoint `POST /api/github/promote`

---

## 13. Stack technique

### Backend (Rust)
- **Web** : Axum 0.7 + Tokio + Tower
- **Base de données** : SQLx 0.8 (SQLite + PostgreSQL), Rusqlite
- **Observabilité** : Prometheus, OpenTelemetry OTLP, Tracing
- **Cloud** : AWS SDK (Bedrock, STS), Azure SDK, Kube
- **Sécurité** : Ring, jsonwebtoken, regex

### Frontend (TypeScript)
- **UI** : React 19, React Router 7, TailwindCSS 3
- **État** : TanStack Query 5
- **Graphiques** : Recharts, date-fns
- **Build** : Vite 8, TypeScript 6
- **Monitoring** : OpenObserve RUM

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   pylos-server                       │
│           Axum HTTP server, routes, middleware       │
├─────────────────────────────────────────────────────┤
│                 pylos-application                     │
│     Use cases, orchestration, stores, plugins        │
├─────────────────────────────────────────────────────┤
│               pylos-infrastructure                   │
│      Provider adapters (OpenAI, Anthropic, etc.)     │
├─────────────────────────────────────────────────────┤
│                   pylos-core                         │
│         Domain entities, traits, config types        │
└─────────────────────────────────────────────────────┘
```

Architecture hexagonale (ports & adapters) en 4 crates Rust.

---

## Points clés pour la présentation vidéo

1. **Valeur principale** : Unifier tous les fournisseurs LLM derrière une API unique compatible OpenAI
2. **Gouvernance** : Contrôle d'accès granulaire, rate limiting, budgets — indispensable pour les équipes
3. **Observabilité** : Métriques, tracing, logs — tout est traçable et mesurable
4. **Rust** : Performance, sécurité mémoire, faible latence
5. **Plugins** : Architecture extensible pour la sécurité, le caching, le RAG
6. **Déploiement** : Docker, Kubernetes, CI/CD prêts à l'emploi
7. **Interface React** : Dashboard complet pour l'administration sans ligne de commande
