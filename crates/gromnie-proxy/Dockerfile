FROM rust:1.91.1 AS base

RUN cargo install cargo-chef

ENV CARGO_HOME=/usr/local/cargo

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# ── Native build (cargo-chef) ──────────────────────────────────────────

FROM base AS chef

WORKDIR /app
COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM base AS cacher

WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json

RUN --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    cargo chef cook --release -p gromnie-proxy --recipe-path recipe.json

FROM base AS builder

WORKDIR /app

COPY --from=cacher /app/target target
COPY --from=cacher /app/Cargo.lock Cargo.lock
COPY . .

ARG GIT_SHA=dev
ENV GIT_SHA=${GIT_SHA}

RUN --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    cargo build --release -p gromnie-proxy

# ── Runtime ─────────────────────────────────────────────────────────────

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gromnie-proxy /app/gromnie-proxy

EXPOSE 8080

CMD ["./gromnie-proxy"]
