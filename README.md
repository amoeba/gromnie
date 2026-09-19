# gromnie

gromnie is a headless, cross-platform [Asheron's Call](https://en.wikipedia.org/wiki/Asheron%27s_Call) game
client written in Rust. gromnie depends heavily on [asheron-rs](https://github.com/amoeba/asheron-rs), my AC protocol and .dat library (also in Rust).

## Background

gromnie started as an attempt to port parad0x's [actestclient](https://github.com/paradoxlost/actestclient) to Rust back in early 2024. At that point, I was only able to get login working. Then, around the time GPT 3.5 came out, I was able to continue making headway on features.

gromnie wouldn't have been possible without existing sources like actestclient, trevis' various libraries, and the ACE source.

## Features

1. Headless client library for use in other projects
2. Wasm-based scripting system with hot reload (`gromnie-scripting`)
3. Basic CLI client (`gromnie-cli`)
3. Basic TUI client (`gromnie-tui`)
4. Discord bot (`discord-bot`)
5. Load tester (`load-tester`) for spawning potentially infinite clients at once.
6. Browser client with UDP proxy.

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

## Development

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
## Acknowledgements

- Protocol types and wire format are provided by
  [`asheron-rs`](https://github.com/amoeba/asheron-rs).
- Browser tunneling builds on [`wisp-mux`](https://github.com/MercuryWorkshop/epoxy-tls) and
  the WISP protocol from the Mercury Workshop.
- Asheron's Call is a trademark of its respective owners. This project is an independent,
  community-built client and is not affiliated with or endorsed by them.

## License

Licensed under the [MIT License](LICENSE).
