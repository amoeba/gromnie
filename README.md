# gromnie

**gromnie is a headless [Asheron's Call](https://en.wikipedia.org/wiki/Asheron%27s_Call) game
client written in Rust.**

It implements the AC network protocol in Rust — the authentication and login handshake,
character selection, world entry, and dispatch of the server-to-client message and game-event
streams — without driving the retail graphical client. A reusable client library sits at the
core, with a terminal UI, a WASM scripting runtime, a Discord bot, a load tester, and a
browser/WebSocket transport built on top.

## Features

1. **Headless protocol client** — `gromnie-client` implements the login/auth handshake,
   character selection, world entry, and server-to-client message dispatch using
   [`asheron-rs`](https://github.com/amoeba/asheron-rs) message types.
2. **Pluggable event consumers** — `gromnie-events` and `gromnie-runner` deliver game, protocol,
   state, and system events to consumers such as the TUI, logging, stats, auto-login, Discord,
   and scripts.
3. **WASM scripting** — scripts compile to WASM components and run in Wasmtime; they receive
   typed events, read client-state snapshots, and send actions (`SendChatSay`, `SendChatTell`,
   `LoginCharacter`, `DoMovementCommand`, `StopMovementCommand`, `Disconnect`, …) back to the
   client. Supports hot reload and per-script execution timeouts.
4. **Terminal UI** — the `tui` binary is an interactive `ratatui` client.
5. **Headless CLI** — the `cli` binary runs the client without a UI.
6. **Discord bot** — the `discord-bot` binary forwards messages between a Discord channel and
   in-game chat.
7. **Load testing** — the `load-tester` binary spawns many concurrent clients against a server
   and reports aggregate stats.
8. **Browser client** — `gromnie-web` compiles the client to WASM via `wasm-bindgen` and
   tunnels UDP over WebSockets.
9. **WISP proxy** — `gromnie-proxy` provides the WISP-over-WebSocket → UDP bridge used by the
   browser client.
10. **Cross-platform** — CI runs the test suite on Linux, macOS, and Windows.

## Project layout

The repository is a Cargo workspace. The most important crates:

| Crate | Description |
| --- | --- |
| `gromnie-client` | Core client library: protocol state machine, scenes, native + WASM transports. |
| `gromnie-events` | Event types (`SimpleGameEvent`, `ProtocolEvent`, client/system/state events) and the `EventConsumer` abstraction. |
| `gromnie-runner` | Runs clients, wires up event consumers, multi-client/load-test orchestration, and logging. |
| `gromnie-scripting-api` | Guest-side API (WIT + Rust) that scripts are written against. |
| `gromnie-scripting-host` | Host runtime that loads/runs WASM scripts in Wasmtime (async, timers, hot reload). |
| `gromnie-tui` | The `tui` terminal client binary. |
| `gromnie-cli` | The `cli`, `discord-bot`, and `load-tester` binaries. |
| `gromnie-proxy` | WISP proxy server: WebSocket ⇄ UDP, used to reach AC servers from the browser. |
| `gromnie-wisp` | Shared WISP protocol helpers used by the proxy and web client. |
| `gromnie-web` | `wasm-bindgen` browser bindings plus a demo UI. |
| `xtask` | Build automation (`cargo xtask ...`). |

### Binaries

| Binary | Crate | What it does |
| --- | --- | --- |
| `tui` | `gromnie-tui` | Interactive terminal client. |
| `cli` | `gromnie-cli` | Headless command-line client. |
| `discord-bot` | `gromnie-cli` | Bridges a Discord channel and the in-game chat. |
| `load-tester` | `gromnie-cli` | Spawns many clients to load-test an AC server. |
| `gromnie-proxy` | `gromnie-proxy` | WISP-over-WebSocket proxy for the browser client. |

### Event flow

```
AC server ──UDP──▶ gromnie-client ──▶ gromnie-events ──▶ EventConsumer
                                          │                 ├─ TUI
                                          │                 ├─ logging
                                          │                 ├─ Discord
                                          │                 ├─ stats / load tester
                                          ──▶ ProtocolEvent ──▶ WASM scripts
```

For the browser path, UDP is tunneled over a WebSocket:

```
Browser (WASM) ──wss──▶ reverse proxy ──▶ gromnie-proxy ──UDP──▶ AC server
```

## Getting a dev environment up

### Prerequisites

- **Rust stable** (edition 2024, so Rust **1.85+**). Install via [rustup](https://rustup.rs/).
- **Git**.
- Optional, depending on what you work on:
  - **[prek](https://github.com/j178/prek)** for git hooks (recommended).
  - **`wasm32-wasip2` target + `wasm-tools`** for building scripts.
  - **`wasm32-unknown-unknown` target + `wasm-bindgen-cli` + Node.js** for the web client.

### Clone and build

```bash
git clone https://github.com/amoeba/gromnie.git
cd gromnie

# Build everything
cargo build --workspace

# Or a release build
cargo build --workspace --release
```

### Configure a server and account

The `cli` binary writes an example config on first run. On macOS/Linux the config lives at
`~/.config/gromnie/config.toml` (XDG-aware); on Windows it is under `%APPDATA%\gromnie\`.

```bash
cargo run --bin cli        # creates the example config if missing, then exits
```

Then edit `~/.config/gromnie/config.toml`:

```toml
[servers.local]
host = "localhost"
port = 9000

[accounts.default]
username = "user"
password = "pass"
# character = "MyCharacterName"   # optional auto-login
```

### Run something

```bash
# Terminal UI (config-based)
cargo run --bin tui -- --server local --account default

# Terminal UI (direct connection)
cargo run --bin tui -- --host play.example.com --port 9000 \
  --account myaccount --password mypassword --character MyCharacter

# Headless CLI
cargo run --bin cli -- --server local --account default

# Load tester (5 clients against localhost:9000)
cargo run --bin load-tester -- --clients 5 --host localhost --port 9000
```

Logging goes to stdout. Set `GROMNIE_LOG_FILE=1` to also write to
`~/.local/share/gromnie/logs/<component>.log`. Use `RUST_LOG` to control verbosity.

### Working on scripts

Scripts are WASM components written against `gromnie-scripting-api`. See
[`docs/scripting.md`](docs/scripting.md) for the API and examples.

```bash
rustup target add wasm32-wasip2
cargo install wasm-tools

cargo xtask scripts build     # build scripts from ./scripts
cargo xtask scripts install   # install to ~/.local/share/gromnie/scripts
```

Scripting is configured under `[scripting]` in `config.toml` (enabled, script directory,
hot reload, per-script config, execution timeout).

### Working on the web client

The browser client is more involved. Full instructions live in
[`crates/gromnie-web/agents.md`](crates/gromnie-web/agents.md); the short version:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

cargo xtask web build         # produces crates/gromnie-web/pkg/
cd crates/gromnie-web && npm install && npm run dev
```

There is also a `docker compose up --build` path that runs the web app and the WISP proxy
together. See [`crates/gromnie-web/README.md`](crates/gromnie-web/README.md) for the JS/TS API
and network architecture.

## Running tests

The CI pipeline is the source of truth. From the repository root:

```bash
# Formatting
cargo fmt --all -- --check

# Compile check + lints (warnings are errors in CI)
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings

# Unit tests
cargo test --lib --all-features

# Integration tests
cargo test --test '*' --all-features

# Everything at once
cargo test --workspace --all-features
```

Run a single crate's tests while iterating:

```bash
cargo test -p gromnie-client
cargo test -p gromnie-scripting-host
```

Scripting integration tests load the prebuilt WASM fixture
`tests/scripting/test_script.wasm`, so no extra toolchain is required to run them.

## Contributing

Contributions are welcome. The general flow:

1. **Fork / branch.** Create a topic branch off `main`, e.g.
   `git checkout -b feat/my-change`.
2. **Make your change**, and add or update tests where it makes sense.
3. **Run the checks** locally (see below). CI runs formatting, clippy, `cargo check`, and the
   test suite on Linux, macOS, and Windows.
4. **Commit** with a clear message.
5. **Open a pull request** against `main` and describe what changed and why.

### Set up the git hooks

This repo uses [prek](https://github.com/j178/prek) to run `cargo fmt`, `cargo clippy`, and a
few hygiene checks on every commit:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/j178/prek/releases/latest/download/prek-installer.sh | sh
prek install
```

Run them manually at any time:

```bash
prek run --all-files
```

If a hook reformats files, re-stage and amend:

```bash
git add -u && git commit --amend --no-edit
```

### Manual checks

If prek isn't available, run the equivalent checks directly:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

Auto-fix formatting and lints:

```bash
cargo fmt
cargo clippy --all-targets --all-features --fix --allow-dirty
```

### Guidelines

- Match the existing style; `cargo fmt` and clippy must be clean.
- Keep changes focused; prefer small, reviewable PRs.
- Update documentation when behavior or public APIs change. Relevant docs:
  - [`docs/scripting.md`](docs/scripting.md) — scripting API
  - [`docs/acnetworkprotocol.md`](docs/acnetworkprotocol.md) — protocol notes
  - [`ASYNC.md`](ASYNC.md) — async WASM scripting design
  - [`agents.md`](agents.md) — instructions for AI agents
- AI agents working in this repo should read [`agents.md`](agents.md) and the crate-specific
  [`crates/gromnie-web/agents.md`](crates/gromnie-web/agents.md).

## Acknowledgements

- Protocol types and wire format are provided by
  [`asheron-rs`](https://github.com/amoeba/asheron-rs).
- Browser tunneling builds on [`wisp-mux`](https://github.com/MercuryWorkshop/epoxy-tls) and
  the WISP protocol from the Mercury Workshop.
- Asheron's Call is a trademark of its respective owners. This project is an independent,
  community-built client and is not affiliated with or endorsed by them.

## License

Licensed under the [MIT License](LICENSE).