# ── Build WASM artifacts ──────────────────────────────────────────────
# Uses the same Rust base as the proxy Dockerfile for consistency.
FROM rust:1.91.1 AS base

RUN cargo install cargo-chef
ENV CARGO_HOME=/usr/local/cargo
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install wasm32 target and wasm-bindgen-cli in the base stage so they
# are available in all subsequent stages.
RUN rustup target add wasm32-unknown-unknown && \
    cargo install wasm-bindgen-cli --version 0.2.120 --root /usr/local

FROM base AS chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS cacher
WORKDIR /app
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    cargo chef cook --release -p gromnie-web --target wasm32-unknown-unknown --recipe-path recipe.json

FROM base AS builder
WORKDIR /app
COPY --from=cacher /app/target target
COPY --from=cacher /app/Cargo.lock Cargo.lock
COPY . .

# Build the WASM package and generate JS glue + wrapper files
RUN --mount=type=cache,target=$CARGO_HOME/registry,sharing=locked \
    --mount=type=cache,target=$CARGO_HOME/git,sharing=locked \
    PATH="/usr/local/bin:$PATH" cargo xtask web build

# ── Runtime: static file server ───────────────────────────────────────
FROM nginx:1.27-alpine

# Default WISP proxy host (overridden via WISP_HOST env var at runtime).
ENV WISP_HOST=gromnie-proxy:8080

# Copy demo files and generated pkg/ directory. The index.html still contains
# the __WISP_HOST__ placeholder, which the entrypoint replaces at startup.
COPY --from=builder /app/crates/gromnie-web/demo/index.html /usr/share/nginx/html/index.html
COPY --from=builder /app/crates/gromnie-web/demo/main.js  /usr/share/nginx/html/main.js
COPY --from=builder /app/crates/gromnie-web/pkg/           /usr/share/nginx/html/pkg/

# Entrypoint wrapper: substitute __WISP_HOST__ with the env var, then
# hand off to the standard nginx entrypoint.
COPY <<'ENTRYPINT' /usr/local/bin/entrypoint.sh
#!/bin/sh
sed -i "s/__WISP_HOST__/${WISP_HOST}/g" /usr/share/nginx/html/index.html
exec /docker-entrypoint.sh nginx -g 'daemon off;'
ENTRYPINT
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Replace default config with one that listens on 8080 (matches Dokku port mapping).
# Remove the IPv6 entrypoint script that would revert our port change.
RUN rm -f /docker-entrypoint.d/10-listen-on-ipv6-by-default.sh && \
    rm -f /etc/nginx/conf.d/default.conf && \
    cat > /etc/nginx/conf.d/default.conf <<'EOF'
server {
    listen 8080;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;
    include /etc/nginx/mime.types;
    types {
        application/javascript mjs;
        application/wasm wasm;
    }
    location / {
        try_files $uri $uri/ =404;
    }
}
EOF

EXPOSE 8080
