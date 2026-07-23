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

ARG GIT_SHA=dev
ENV GIT_SHA=${GIT_SHA}

# Proxy upstream — defaults to Docker Compose service name, override for
# Dokku / external deployments (e.g. gromnie-proxy.treestats.net).
ENV PROXY_UPSTREAM=gromnie-proxy:8081

# Copy demo files and generated pkg/ directory.
COPY --from=builder /app/crates/gromnie-web/demo/index.html /usr/share/nginx/html/index.html
COPY --from=builder /app/crates/gromnie-web/demo/main.js    /usr/share/nginx/html/main.js
COPY --from=builder /app/crates/gromnie-web/demo/css/       /usr/share/nginx/html/css/
COPY --from=builder /app/crates/gromnie-web/demo/components/ /usr/share/nginx/html/components/
COPY --from=builder /app/crates/gromnie-web/demo/public/worker.js  /usr/share/nginx/html/worker.js
COPY --from=builder /app/crates/gromnie-web/pkg/            /usr/share/nginx/html/pkg/

# Entrypoint wrapper: substitute env vars in nginx config + static files,
# then hand off to the standard nginx entrypoint.
COPY <<'ENTRYPOINT' /usr/local/bin/entrypoint.sh
#!/bin/sh
envsubst '${PROXY_UPSTREAM}' < /etc/nginx/conf.d/default.conf.template > /etc/nginx/conf.d/default.conf
sed -i "s/__GIT_SHA__/${GIT_SHA}/g" /usr/share/nginx/html/index.html
sed -i "s/__GIT_SHA__/${GIT_SHA}/g" /usr/share/nginx/html/main.js
sed -i "s/__GIT_SHA__/${GIT_SHA}/g" /usr/share/nginx/html/components/status-bar.js
exec /docker-entrypoint.sh nginx -g 'daemon off;'
ENTRYPOINT
RUN chmod +x /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# Replace default config: listen on 8080, proxy /wisp to the upstream.
# Uses a .template so envsubst can swap ${PROXY_UPSTREAM} at runtime.
# Remove the IPv6 entrypoint script that would revert our port change.
RUN rm -f /docker-entrypoint.d/10-listen-on-ipv6-by-default.sh && \
    rm -f /etc/nginx/conf.d/default.conf && \
    cat > /etc/nginx/conf.d/default.conf.template <<'EOF'
server {
    listen 8080;
    server_name _;
    root /usr/share/nginx/html;
    index index.html;
    include /etc/nginx/mime.types;
    types {
        application/javascript mjs;
    }
    location / {
        try_files $uri $uri/ =404;
    }
    location /auth {
        proxy_pass https://${PROXY_UPSTREAM};
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_ssl_server_name on;
    }
    location /wisp {
        proxy_pass https://${PROXY_UPSTREAM};
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_ssl_server_name on;
        proxy_read_timeout 86400;
    }
}
EOF

EXPOSE 8080
