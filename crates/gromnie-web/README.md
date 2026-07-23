# gromnie-web

`gromnie-web` is a wasm-focused crate that exposes a `wasm-bindgen` API for
connecting to an Asheron's Call game server through a WISP-over-WebSocket
proxy. It wraps the `gromnie-client` protocol implementation with a browser
transport layer.

## Build

```bash
cargo xtask web build
```

This produces browser artifacts in `crates/gromnie-web/pkg/` via `cargo build` +
`wasm-bindgen`.

## Usage (JS/TS)

The public API is the `GromnieClient` class, which handles the full connection
flow: WISP handshake, UDP stream tunneling, AC protocol login, and event
delivery.

The WASM module is auto-initialized when you import from `index.mjs`, so there's
no need to call `init()` separately.

```ts
import { GromnieClient } from "gromnie-web";

const client = new GromnieClient("wss://example.com/wisp/");

// Receive game events
client.set_on_event((event) => {
  console.log("event:", event);
});

// Receive network log entries
client.set_on_net_log((entry) => {
  console.log("net:", entry);
});

// Connect through the WISP proxy to the game server
await client.connect(
  "play.coldeve.ac",   // game server host
  9000,                // game server port
  "account",           // account name
  "password"           // account password
);

// After the character list arrives, select a character to enter the world
client.select_character(characterId);

// Send a chat message
client.send_chat("hello world");

// Disconnect
await client.disconnect();
```

### Events

Events are delivered as structured JS objects via the `on_event` callback. Each
event has a `type` field that discriminates the variant:

```ts
// Game events
{ type: "CharacterListReceived", account: "...", characters: [...], num_slots: 11 }
{ type: "ChatMessageReceived", message: "...", message_type: 0 }
{ type: "LoginSucceeded", character_id: 1, character_name: "..." }

// State events
{ type: "Connecting" }
{ type: "CharacterSelect" }
{ type: "InWorld" }

// System events
{ type: "Disconnected", will_reconnect: false, reconnect_attempt: 0, delay_secs: 0 }
{ type: "AuthenticationFailed", reason: "..." }
```

### Internal transport

The WISP-over-WebSocket transport layer (`GromnieWispClient`) is an internal
implementation detail of `GromnieClient`. It is not exported to JavaScript.
