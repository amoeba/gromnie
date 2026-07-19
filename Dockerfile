FROM rust:1.82 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p gromnie-proxy

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gromnie-proxy /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["gromnie-proxy"]
