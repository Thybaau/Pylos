.PHONY: setup run build test lint fmt audit deny all clean

# ── Setup ─────────────────────────────────────────────────────────────────────
## Installe les outils de développement nécessaires
setup:
	rustup component add clippy rustfmt
	@echo "Installing cargo-audit (security audits)..."
	cargo install cargo-audit --quiet || true
	@echo "Installing cargo-deny (dependency policy)..."
	cargo install cargo-deny --quiet || true
	@echo "Setup complete."

# ── Build ─────────────────────────────────────────────────────────────────────
## Compile en mode release
build:
	cargo build --release -p pylos-server

## Compile en mode debug (rapide)
build-dev:
	cargo build -p pylos-server

# ── Run ───────────────────────────────────────────────────────────────────────
## Lance le serveur en mode dev (rechargement manuel)
run:
	RUST_LOG=info cargo run -p pylos-server

## Lance le serveur avec un fichier de config spécifique
run-config:
	RUST_LOG=info cargo run -p pylos-server -- --config pylos.json

# ── Test ──────────────────────────────────────────────────────────────────────
## Lance tous les tests unitaires et d'intégration
test:
	cargo test

## Lance les tests avec output verbose
test-verbose:
	cargo test -- --nocapture

# ── Lint & Format ─────────────────────────────────────────────────────────────
## Applique le formattage automatique
fmt:
	cargo fmt --all

## Vérifie le formattage sans modification
fmt-check:
	cargo fmt --all -- --check

## Lance clippy (lint)
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# ── Sécurité ──────────────────────────────────────────────────────────────────
## Audit de sécurité des dépendances (requiert cargo-audit)
audit:
	cargo audit

## Vérification des politiques de dépendances (requiert cargo-deny)
deny:
	cargo deny check

# ── Docker ────────────────────────────────────────────────────────────────────
## Build l'image Docker
docker-build:
	docker build -t pylos:latest .

## Lance la stack complète (gateway + UI + Prometheus + Grafana)
docker-up:
	docker compose up --build

## Arrête la stack
docker-down:
	docker compose down

# ── UI ────────────────────────────────────────────────────────────────────────
## Lance le serveur de dev UI
ui-dev:
	cd ui && npm run dev

## Build la UI pour la production
ui-build:
	cd ui && npm run build

# ── Clean ─────────────────────────────────────────────────────────────────────
## Nettoie les artefacts de compilation
clean:
	cargo clean

# ── Pipeline CI/CD ────────────────────────────────────────────────────────────
## Pipeline complet : format + lint + test
all: fmt-check lint test
	@echo "All checks passed."
