# gromnie-web

`gromnie-web` is a wasm-focused crate that exposes a small `wasm-bindgen` API for opening WISP multiplexed streams over a browser `WebSocket`.

## Build

```bash
cargo xtask web build
```

This produces deterministic browser artifacts in `crates/gromnie-web/pkg/` via `cargo build` + `wasm-bindgen`.

## Usage (JS/TS)

```ts
import init, { GromnieWispClient } from "gromnie-web";

await init();
const client = new GromnieWispClient("wss://example.com/wisp/");
await client.connect();

const tcpStreamId = await client.open_tcp_stream("example.com", 443);
const udpStreamId = await client.open_udp_stream("1.1.1.1", 53);
```

The MVP currently returns stream IDs and keeps streams alive internally to preserve mux lifecycle.
