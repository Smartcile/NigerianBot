# syntax=docker/dockerfile:1

# ─── Build stage ───────────────────────────────────────────────────────────
# Build context is the repo ROOT (see docker-compose.yml), so the whole Cargo
# workspace is available and `common` resolves correctly.
FROM rust:1-bookworm AS builder
WORKDIR /app

# Copy the workspace. Only `-p bot` (and its deps) actually gets compiled.
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY bot ./bot
COPY api ./api
COPY scheduler ./scheduler
COPY worker ./worker

RUN cargo build --release -p bot

# ─── Runtime stage ─────────────────────────────────────────────────────────
# Slim image; ca-certificates is required for TLS to the Discord gateway/API.
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nigerian-bot /usr/local/bin/nigerian-bot

# Non-root for safety.
RUN useradd --create-home --uid 10001 appuser
USER appuser

ENTRYPOINT ["/usr/local/bin/nigerian-bot"]
