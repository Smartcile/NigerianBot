# syntax=docker/dockerfile:1
#
# All-in-one NigerianBot image: one binary (`nigerianbot`) runs the Discord bot,
# the API + dashboard, the download worker, and the Telegram bot in-process.
#
# Runtime needs: ca-certificates (TLS), ffmpeg + yt-dlp (music/downloads),
# libopus0 (Discord voice). Build needs cmake/libopus-dev for songbird.

# ─── Frontend (React/Vite) ──────────────────────────────────────────────────
FROM node:20-bookworm-slim AS frontend
WORKDIR /dash
COPY dashboard/package.json ./
RUN npm install
COPY dashboard/ ./
RUN npm run build

# ─── Rust ───────────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# songbird's audio stack (opus) needs cmake + libopus to build.
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake libopus-dev pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
# This layer is the slow one — cached unless Cargo.toml/Cargo.lock change.
RUN cargo chef cook --release --recipe-path recipe.json -p app
# Copy ONLY Rust inputs (not dashboard/docs). A dashboard or docs edit then
# rebuilds just the frontend stage below instead of recompiling every crate.
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY app ./app
COPY bot ./bot
COPY api ./api
COPY worker ./worker
COPY telegram ./telegram
COPY scheduler ./scheduler
COPY migrations ./migrations
RUN cargo build --release -p app

# ─── Runtime stage ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg libopus0 curl \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux \
        -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nigerianbot /usr/local/bin/nigerianbot
# Built dashboard, served by the API at "/".
COPY --from=frontend /dash/dist /app/static

# Seed the downloads dir owned by appuser so a FRESH named volume inherits write
# permission for the non-root user.
RUN useradd --create-home --uid 10001 appuser \
    && mkdir -p /downloads \
    && chown 10001:10001 /downloads
USER appuser

EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/nigerianbot"]
