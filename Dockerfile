# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — Builder
# Utilise l'image officielle Rust avec les dépendances système nécessaires
# (openssl, pkg-config) pour compiler le workspace
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1.95-bookworm AS builder

WORKDIR /build

# Dépendances système pour openssl (requis par reqwest)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# ─── Couche cache des dépendances ────────────────────────────────────────────
# On copie uniquement les manifestes d'abord pour bénéficier du cache Docker.
# Tant que Cargo.toml/Cargo.lock ne changent pas, cette couche est réutilisée.
COPY Cargo.toml Cargo.lock ./

# Création de stubs vides pour chaque crate du workspace
RUN mkdir -p \
    crates/pylos-core/src \
    crates/pylos-application/src \
    crates/pylos-infrastructure/src \
    crates/pylos-server/src

COPY crates/pylos-core/Cargo.toml         crates/pylos-core/Cargo.toml
COPY crates/pylos-application/Cargo.toml  crates/pylos-application/Cargo.toml
COPY crates/pylos-infrastructure/Cargo.toml crates/pylos-infrastructure/Cargo.toml
COPY crates/pylos-server/Cargo.toml       crates/pylos-server/Cargo.toml

# Stubs minimaux pour pré-compiler les dépendances
RUN echo "fn main() {}" > crates/pylos-server/src/main.rs && \
    echo "pub fn init() {}" > crates/pylos-core/src/lib.rs && \
    echo "pub fn init() {}" > crates/pylos-application/src/lib.rs && \
    echo "pub fn init() {}" > crates/pylos-infrastructure/src/lib.rs

# Pré-compilation des dépendances uniquement (layer cachée)
RUN cargo build --release -p pylos-server 2>&1 | tail -5 || true

# ─── Build réel ──────────────────────────────────────────────────────────────
# On supprime les artefacts des stubs pour forcer la recompilation du code réel
RUN rm -rf \
    target/release/.fingerprint/pylos-* \
    target/release/deps/pylos_* \
    target/release/pylos-*

# Copie de tout le code source
COPY crates/ crates/
COPY rustfmt.toml ./

# Build de release
RUN cargo build --release -p pylos-server

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — Runtime
# Image minimale Debian slim : accès aux libs système (libssl, libc) sans Rust
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Dépendances runtime uniquement
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copie du binaire compilé
COPY --from=builder /build/target/release/pylos-server /app/pylos-server

# Utilisateur non-root pour la sécurité
RUN useradd --system --uid 1001 --no-create-home pylos
USER pylos

# Variables d'environnement par défaut
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/app/pylos-server"]
