# gromnie-web — Agent Testing Guide

This document describes how to build, run, and test `gromnie-web` locally using
`chrome-devtools-mcp` to drive a headless/headful Chrome browser.

## Prerequisites

- Rust toolchain with the `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)
- Node.js (optional, for the headless harness in `demo/harness/`)
- Google Chrome or Chromium
- The `gromnie-proxy` binary (built from `crates/gromnie-proxy`)

## 1. Build the WASM artifacts

From the repo root:

```bash
cargo xtask web build
```

This produces:

- `crates/gromnie-web/pkg/gromnie_web.js`
- `crates/gromnie-web/pkg/gromnie_web_bg.wasm`
- `crates/gromnie-web/pkg/gromnie_web.d.ts`

The demo (`demo/main.js`) imports `../pkg/gromnie_web.js`, so the pkg output
must exist before the demo can load in a browser.

## 2. Set up credentials

Credentials are stored in `.env` (gitignored) so they are never hardcoded in
this file. Copy the template and fill in your values:

```bash
cp crates/gromnie-web/.env.example crates/gromnie-web/.env
```

The `.env` file contains two groups of credentials:

| Variable            | Purpose                                      |
|---------------------|----------------------------------------------|
| `AUTH_ENABLE`       | Set to `true` to enable proxy basic auth     |
| `AUTH_USER`         | Proxy basic-auth username                    |
| `AUTH_PASSWORD`     | Proxy basic-auth password                    |
| `AUTH_SECRET`       | (Optional) HMAC secret for session cookies   |
| `GROMNIE_GAME_HOST` | AC game server hostname (e.g. `play.coldeve.ac`) |
| `GROMNIE_GAME_PORT` | AC game server port (e.g. `9000`)            |
| `GROMNIE_GAME_ACCOUNT` | Account name for the game server          |
| `GROMNIE_GAME_PASSWORD` | Password for the game server              |

## 3. Build and start the proxy

The proxy serves both the demo static files and the WISP WebSocket endpoint
at `/wisp/`. Build it once, then start it with env vars sourced from `.env`:

```bash
# Build (once)
cargo build -p gromnie-proxy

# Start the proxy (sources .env for AUTH_* and game credentials)
bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
  ./target/debug/gromnie-proxy \
    --listen 127.0.0.1:8080 \
    --wisp-path /wisp/ \
    --static-dir crates/gromnie-web'
```

The proxy logs should show:

```
INFO gromnie_proxy: basic auth enabled user=<AUTH_USER>
INFO gromnie_proxy: listening listen=127.0.0.1:8080 wisp_path=/wisp/ static_dir=crates/gromnie-web
```

## 4. Test with chrome-devtools-mcp

Open Chrome and navigate to the demo. Because basic auth is enabled, embed the
proxy credentials in the URL:

```
http://<AUTH_USER>:<AUTH_PASSWORD>@127.0.0.1:8080/demo/
```

### Step-by-step MCP tool calls

> Replace `<AUTH_USER>` and `<AUTH_PASSWORD>` with the values from `.env`.

#### 4a. Navigate to the demo

```
chrome-mcp__navigate_page url="http://<AUTH_USER>:<AUTH_PASSWORD>@127.0.0.1:8080/demo/"
```

#### 4b. Verify the page loaded

Take a snapshot and check the status bar:

```
chrome-mcp__take_snapshot
```

Expected:
- **WASM** status: `ready`
- **Proxy** status: `reachable`
- Log shows: `wasm loaded from ../pkg/gromnie_web.js`
- Log shows: `proxy ws://127.0.0.1:8080/wisp/: reachable`

#### 4c. Fill in the login form

Use the game-server credentials from `.env` (`GROMNIE_GAME_HOST`,
`GROMNIE_GAME_PORT`, `GROMNIE_GAME_ACCOUNT`, `GROMNIE_GAME_PASSWORD`).

First take a snapshot to get the element UIDs for the form fields, then:

```
chrome-mcp__fill_form elements=[
  {uid: "<host-uid>",     value: "<GROMNIE_GAME_HOST>"},
  {uid: "<port-uid>",     value: "<GROMNIE_GAME_PORT>"},
  {uid: "<account-uid>",  value: "<GROMNIE_GAME_ACCOUNT>"},
  {uid: "<password-uid>", value: "<GROMNIE_GAME_PASSWORD>"}
]
```

#### 4d. Click Login

```
chrome-mcp__click uid="<login-button-uid>"
```

Expected log output:

```
connecting to ws://127.0.0.1:8080/wisp/...
connected and login sent, waiting for server response...
event: game:CharacterListReceived { account: "<GROMNIE_GAME_ACCOUNT>", characters: [...], num_slots: 11 }
found N character(s)
```

The character-select view should appear with a list of characters and an
**Enter World** button.

#### 4e. Enter the world (optional)

Click the first character in the list, then click **Enter World**:

```
chrome-mcp__click uid="<enter-world-button-uid>"
```

Expected log output:

```
entering world with character: <name> (ID: <id>)...
character selected, entering world...
event: game:ChatMessageReceived { message: "Welcome to Coldeve. ...", message_type: 0 }
event: protocol:S2C(LoginCreatePlayer { character_id: <id> })
event: game:LoginSucceeded { character_id: <id>, character_name: "" }
event: game:CreatePlayer { character_id: <id> }
```

The world view should appear with a chat input and message area.

#### 4f. Check for errors

```
chrome-mcp__list_console_messages types=["error", "warn"]
```

A `404` for `favicon.ico` is expected and harmless. A warning about
`sleep(1s) is a no-op on WASM` is also expected.

## 5. Headless harness (alternative)

For CI or automated runs without chrome-devtools-mcp, use the built-in
headless harness:

```bash
cargo xtask web build
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs
```

This serves the project root, launches headless Chrome, and asserts that the
WASM module loads. Add `--scenario=connect-flow` to test the connect + open
TCP/UDP flow.

## 6. Quick reference

```bash
# 1. Build WASM
cargo xtask web build

# 2. Build proxy
cargo build -p gromnie-proxy

# 3. Start proxy (sources .env)
bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
  ./target/debug/gromnie-proxy --listen 127.0.0.1:8080 --wisp-path /wisp/ --static-dir crates/gromnie-web'

# 4. In Chrome (via chrome-devtools-mcp):
#    - Navigate to http://<AUTH_USER>:<AUTH_PASSWORD>@127.0.0.1:8080/demo/
#    - Fill form with GROMNIE_GAME_* credentials
#    - Click Login → observe character list
#    - Click Enter World → observe game events
```
