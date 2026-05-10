# Pylos — Project Context for BMAD

## Project Overview

Pylos is a **high-performance, ultra-low-latency AI Gateway and MCP proxy** written in Rust.
It acts as a unified API facade over multiple LLM providers, presenting an OpenAI-compatible
interface (`POST /v1/chat/completions`) to clients while routing requests to:

- OpenAI
- Anthropic
- AWS Bedrock (with IAM role assumption via STS)
- Ollama (local)
- OpenRouter
- Any OpenAI-compatible endpoint (Groq, vLLM, etc.)

## Architecture

### Backend — Hexagonal / Clean Architecture (Rust)

4-crate Cargo workspace:

| Crate | Role |
|---|---|
| `pylos-core` | Domain layer — pure types, traits, no I/O |
| `pylos-infrastructure` | Provider adapters (OpenAI, Anthropic, Bedrock) |
| `pylos-application` | Use cases: inference orchestration, log store, config hot-reload |
| `pylos-server` | HTTP server (Axum), routes, middleware, metrics |

### Frontend — React / TypeScript

Located in `ui/`:

- React 19 + TypeScript 6 + Vite 8
- TailwindCSS 3
- TanStack Query v5
- Recharts 3 for time-series charts
- 5 pages: Dashboard, Playground, Logs, Providers, Virtual Keys

### Infrastructure

- Docker + Docker Compose (gateway + UI + Prometheus + Grafana)
- GitHub Actions CI/CD
- Pre-commit hooks (fmt, clippy, gitleaks)

## Tech Stack

| Layer | Technology |
|---|---|
| Language (backend) | Rust 2021 edition |
| Async runtime | Tokio 1.36 |
| HTTP framework | Axum 0.7 |
| Serialization | Serde / serde_json |
| HTTP client | reqwest 0.11 |
| Error handling | anyhow + thiserror |
| Observability | Prometheus + tracing |
| AWS | aws-config, aws-sdk-bedrockruntime, aws-sdk-sts |
| UI | React 19 + TypeScript 6 + Vite 8 + TailwindCSS 3 |
| Containerization | Docker multi-stage + Docker Compose |
| Monitoring | Prometheus + Grafana |

## Key Files

- `pylos.json` — Main gateway config (providers, virtual keys, plugins, governance)
- `crates/pylos-core/src/domain/config.rs` — Full config schema
- `crates/pylos-core/src/domain/traits.rs` — `Provider` and `LlmPlugin` traits
- `crates/pylos-core/src/error.rs` — `PylosError` enum
- `crates/pylos-application/src/use_cases/inference.rs` — `InferenceOrchestrator`
- `crates/pylos-server/src/routes.rs` — All HTTP routes

## Key Patterns

- **Provider selection**: smart routing by model name prefix (bedrock→Bedrock, gpt-*→OpenAI, claude-*→Anthropic)
- **Retry with exponential backoff + jitter** per provider
- **Multi-provider fallback**: tries next provider on failure
- **Virtual Keys**: `Bearer sk-pylos-*` tokens with rate limits, budgets, model ACLs
- **Plugin system**: pre/post hooks via `LlmPlugin` trait (FIFO pre, LIFO post)
- **Config hot-reload**: `POST /config/reload` without restart
- **Streaming**: SSE (`text/event-stream`), fully OpenAI-compatible

## API Endpoints

| Method | Path | Description |
|---|---|---|
| POST | /v1/chat/completions | Inference (VK-protected) |
| GET | /v1/models | Models catalog |
| GET | /health | Health check |
| GET | /metrics | Prometheus metrics |
| GET | /api/logs | Request logs |
| GET | /api/logs/stats | Aggregated stats |
| GET | /api/logs/histogram | Request volume histogram |
| POST | /config/reload | Hot-reload config |
| GET/PUT | /providers/:name | Provider management |
| GET | /virtual-keys | Virtual key management |

## Development Conventions

- Run all tests: `cargo test`
- Format: `cargo fmt`
- Lint: `cargo clippy -- -D warnings`
- Build: `cargo build --release`
- Local stack: `docker compose up`
- Config: edit `pylos.json`, reload via `POST /config/reload`
