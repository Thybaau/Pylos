# Pylos — Rust LLM Gateway & MCP Proxy

Pylos is a high-performance, ultra-low latency AI gateway rewritten from scratch in Rust. It provides a unified API for 20+ LLM providers as a drop-in replacement for OpenAI-compatible SDKs, with built-in governance, observability, and a modern React admin dashboard.

## Features

- **Unified AI Gateway** — Single API endpoint for 20+ providers: OpenAI, Anthropic, AWS Bedrock, Azure OpenAI, Google Gemini, Cohere, Groq, Mistral, Cerebras, Perplexity, Fireworks, xAI, Nebius, Ollama, OpenRouter, Vertex AI, DeepSeek, Lemonade, and custom providers.
- **OpenAI-Compatible API** — Drop-in replacement for existing OpenAI SDKs (`/v1/chat/completions`, `/v1/completions`, `/v1/embeddings`, `/v1/images/generations`, `/v1/models`).
- **Streaming** — SSE streaming with token-level metrics.
- **Smart Routing** — Provider-aware model routing, automatic provider detection from model names, weighted load balancing, CEL-based routing rules.
- **Retry & Fallback** — Exponential backoff with jitter, circuit breaker pattern, multi-provider fallback chains.
- **MCP Proxy** — Connect agents to any MCP-compliant tool server.
- **Virtual Keys** — Scoped API keys (`sk-pylos-*`) with per-provider and per-model access control.
- **Rate Limiting & Budgets** — Configurable RPM/TPM limits and USD-based budgets with configurable reset periods.
- **Guardrails** — PII masking, keyword blocking.
- **Observability** — Prometheus metrics (`/metrics`), OpenTelemetry distributed tracing, request logging (SQLite/PostgreSQL), pre-built Grafana dashboards.
- **Hot-Reload Configuration** — Update providers, keys, and rules without restarting.
- **Admin Dashboard** — React 19 + Vite 8 + TailwindCSS dashboard for managing providers, keys, budgets, guardrails, logs, analytics, and MCP tools.

## Architecture

Pylos follows a hexagonal (ports & adapters) architecture across four Rust crates:

```
┌─────────────────────────────────────────────────────┐
│                   pylos-server                       │
│           Axum HTTP/WS server, routes, middleware    │
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

| Crate | Responsibility |
|---|---|
| `pylos-core` | Domain entities, configuration types, provider traits, error types |
| `pylos-application` | Inference orchestration, config store, log store, virtual key/budget/rate-limit stores, guardrails, semantic cache, RAG plugin, OTel plugin, batching, prompt registry |
| `pylos-infrastructure` | Provider implementations (OpenAI-compatible, Anthropic, Bedrock, Azure, Gemini, Cohere) |
| `pylos-server` | Axum HTTP server, routes, middleware (virtual key auth, management auth, request queuing), Prometheus metrics, OTel setup |

## Quick Start

```bash
# Clone and enter the repo
git clone <repo-url> && cd Pylos

# Install dev tools (clippy, rustfmt, cargo-audit, cargo-deny)
make setup

# Configure environment
cp .env.example .env
# Edit .env with your API keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)

# Run in development mode
make run
```

### Docker Compose (full stack)

```bash
cp .env.example .env
docker compose up -d
```

This starts Pylos (port 3000), the admin UI (port 8080), Prometheus, and Grafana.

## Configuration

Pylos is configured via `pylos.json` at the project root:

- **`providers`** — LLM provider definitions with API keys (`env.VAR` syntax supported), model lists, network settings (timeout, retries, backoff), weighted keys.
- **`governance`** — Virtual keys with per-provider model ACLs, budgets, rate limits, CEL-based routing rules.
- **`server`** — Port, log level, request queuing, CORS, logging settings.
- **`plugins`** — Enable/configure telemetry, logging, semantic cache, guardrails.

Configuration supports hot-reload via `POST /config/reload` or the admin dashboard.

## API Endpoints

### Inference (OpenAI-compatible)

| Method | Path | Description |
|---|---|---|
| `POST` | `/v1/chat/completions` | Chat completions (streaming or non-streaming) |
| `POST` | `/v1/completions` | Text completions |
| `POST` | `/v1/embeddings` | Embedding creation |
| `POST` | `/v1/images/generations` | Image generation |
| `GET` | `/v1/models` | List available models |

### Management (requires `PYLOS_ADMIN_KEY`)

| Method | Path | Description |
|---|---|---|
| `GET/POST/DELETE` | `/providers` | Provider CRUD |
| `POST` | `/providers/:name/test` | Test provider connectivity |
| `GET/POST/PUT/DELETE` | `/virtual-keys` | Virtual key management |
| `GET` | `/virtual-keys/:id/budget` | Virtual key budget usage |
| `GET` | `/config` | Get current configuration |
| `POST` | `/config/reload` | Hot-reload configuration |
| `PUT` | `/config/guardrails` | Update guardrail settings |

### Observability

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/api/logs` | Query request logs |
| `GET` | `/api/logs/stats` | Log statistics |
| `GET` | `/api/logs/histogram` | Time-based log histogram |
| `GET` | `/api/logs/histogram/tokens` | Token usage histogram |

### Usage Example

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="sk-pylos-poc-2024",
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

## Development

```bash
make all          # Full pipeline: format check + lint + test
make test         # Run all unit and integration tests
make fmt          # Apply formatting
make lint         # Clippy with deny warnings
make audit        # Security audit of dependencies
make deny         # Policy check on dependencies
make ui-dev       # Start React UI dev server
```

## Observability

Pylos exposes Prometheus metrics at `/metrics` and supports OpenTelemetry distributed tracing via OTLP. Request logs are stored in SQLite (default) or PostgreSQL with support for filtering, histograms, and analytics. Pre-built Grafana dashboards are included under `docker/grafana/`.

## Deployment

- **Docker** — Multi-arch images (amd64 + arm64) via the included `Dockerfile`.
- **Docker Compose** — Full stack with monitoring in `docker-compose.yml`.
- **Kubernetes** — Helm chart available in `helm/pylos/` with ArgoCD/GitOps support.
- **CI/CD** — GitHub Actions workflows for checking, building, testing, and deploying.

## Security

- Virtual API keys with scoped permissions
- Rate limiting and budget enforcement
- PII masking and keyword blocking guardrails
- Security audits (`make audit`) and dependency deny policies (`make deny`) integrated into CI

## License

TBD
