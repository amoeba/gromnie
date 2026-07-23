# gromnie-web — Agent Testing Guide

This document describes how to build, run, and test `gromnie-web` locally using
`chrome-devtools-mcp` to drive a headless/headful Chrome browser.

## Architecture

- **gromnie-proxy** — Pure WISP proxy (WebSocket → UDP). Listens on `/wisp` by default.
- **gromnie-web** — Vite dev server (or nginx in Docker) serving the WASM demo.
- **SharedWorker** — Holds `GromnieClient` and the WISP connection. Persists across
  page reloads so the game connection survives HMR and full reloads.

```
┌─────────────────┐       postMessage       ┌──────────────────┐
│   Main Page      │ ◄──────────────────► │   SharedWorker    │
│  (UI only)       │                        │  (GromnieClient)  │
│  HTML/CSS/JS     │                        │  WISP connection  │
└─────────────────┘                        └──────────────────┘
        │                                          │
        │  HTTP (Vite or nginx)                    │  WebSocket
        ▼                                          ▼
   Dev server ──proxy /wisp──► gromnie-proxy ──UDP──► AC server
```

In **Docker Compose**, nginx proxies `/wisp` to the gromnie-proxy container so
the browser uses a single origin for everything.

For **local dev**, Vite serves the files and proxies `/wisp` to gromnie-proxy.

### HMR-safe file structure

The codebase is split so that UI edits trigger Vite HMR (hot module replacement)
instead of full page reloads, preserving the SharedWorker connection:

- **`demo/main.js`** — Stable entry point. Creates the SharedWorker and wires
  up the message handler. **Never edit this file during UI development.**
- **`demo/ui.js`** — All UI logic (DOM refs, event handlers, rendering).
  Safe to edit freely — Vite HMRs this module without killing the worker.
- **`demo/public/worker.js`** — SharedWorker script. Served from `public/`
  so Vite passes it through untouched (not transformed as a module).

When you edit `ui.js`, Vite sends an HMR update that swaps the module
without reloading the page. The SharedWorker stays alive and the game
connection persists. The `main.js` entry point accepts the update via
`import.meta.hot.accept()` and rebinds the message handler to the new
module's `handleMessage` function.

**Important:** If you edit `main.js` or `worker.js`, Vite does a full
page reload, which kills the SharedWorker (standard browser behavior).
Avoid editing those files during active development sessions.

## Prerequisites

- Rust toolchain with the `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)
- Node.js (for Vite dev server)
- Google Chrome or Chromium
- The `gromnie-proxy` binary (built from `crates/gromnie-proxy`)
- tmux (for managing dev processes with agents)

## 1. Build the WASM artifacts

From the repo root:

```bash
cargo xtask web build
```

This produces:

- `crates/gromnie-web/pkg/gromnie_web.js`
- `crates/gromnie-web/pkg/gromnie_web_bg.wasm`
- `crates/gromnie-web/pkg/gromnie_web.d.ts`

The demo imports these via `demo/pkg` (a symlink to `../pkg`).

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

## 3. Install npm dependencies

```bash
cd crates/gromnie-web && npm install
```

## 4. Local dev with tmux (recommended)

The easiest way to run the dev environment is with tmux. Create a session with
named windows for each process:

```bash
# From repo root — create a tmux session with proxy + vite
tmux new-session -d -s gromnie-dev -n proxy \
  "bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
    cargo run -p gromnie-proxy -- \
      --listen 127.0.0.1:8081 \
      --wisp-path /wisp; \
    read -p \"Press Enter to exit…\"'"

tmux new-window -t gromnie-dev -n vite \
  "cd crates/gromnie-web && npm run dev; read -p 'Press Enter to exit…'"

tmux attach -t gromnie-dev
```

This gives you two windows: `proxy` (gromnie-proxy) and `vite` (dev server).

### Managing tmux from an agent

Use `tmux send-keys` and `tmux capture-pane` to interact with the processes:

```bash
# Check proxy logs
tmux capture-pane -t gromnie-dev:proxy -p

# Check vite logs
tmux capture-pane -t gromnie-dev:vite -p

# Send a command to the proxy pane
tmux send-keys -t gromnie-proxy "some command" Enter
```

When done:

```bash
tmux kill-session -t gromnie-dev
```

## 5. Docker Compose

The simplest way to run everything without local tooling:

```bash
docker compose up --build -d
```

This starts:
- **gromnie-web** (nginx) on `http://localhost:8080/`
- **gromnie-proxy** on port `8081`

nginx proxies `/wisp` WebSocket connections to the gromnie-proxy container,
so the browser uses a single origin (`localhost:8080`) for everything.

## 6. Test with chrome-devtools-mcp

### Local dev (Vite)

```
chrome-mcp__navigate_page url="http://localhost:5173/demo/"
```

### Docker Compose

```
chrome-mcp__navigate_page url="http://localhost:8080/"
```

### Step-by-step MCP tool calls

#### 6a. Navigate to the demo

```
chrome-mcp__navigate_page url="http://localhost:5173/demo/"
```

#### 6b. Verify the page loaded

Take a snapshot and check the status bar:

```
chrome-mcp__take_snapshot
```

Expected:
- **WASM** status: `ready`
- **Proxy** status: `reachable`
- Log shows: `wasm loaded from ./pkg/index.mjs`
- Log shows: `proxy: reachable`
- Log shows: `worker: not connected` (initial state)

#### 6c. Fill in the login form

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

#### 6d. Click Login

```
chrome-mcp__click uid="<login-button-uid>"
```

Expected log output:

```
connecting to play.coldeve.ac:9000...
connected and login sent, waiting for server response...
event: game:CharacterListReceived { account: "<GROMNIE_GAME_ACCOUNT>", characters: [...], num_slots: 11 }
found N character(s)
```

The character-select view should appear with a list of characters and an
**Enter World** button.

#### 6e. Enter the world (optional)

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

#### 6f. Test reload preservation

Reload the page and verify the SharedWorker reconnects:

```
chrome-mcp__navigate_page type="reload"
chrome-mcp__take_snapshot
```

Expected:
- Log shows: `worker: reconnected to existing session`
- If in world: world view is restored with character name
- If at character select: character list is restored
- **No need to re-enter credentials or re-login**

#### 6g. Check for errors

```
chrome-mcp__list_console_messages types=["error", "warn"]
```

A `404` for `favicon.ico` is expected and harmless. A warning about
`sleep(1s) is a no-op on WASM` is also expected.

## 7. Headless harness (alternative)

For CI or automated runs without chrome-devtools-mcp, use the built-in
headless harness:

```bash
cargo xtask web build
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs
```

This serves the project root, launches headless Chrome, and asserts that the
WASM module loads. Add `--scenario=connect-flow` to test the connect + open
TCP/UDP flow.

## 8. Quick reference

### Local dev (tmux)

```bash
# Start the dev environment
tmux new-session -d -s gromnie-dev -n proxy \
  "bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
    cargo run -p gromnie-proxy -- --listen 127.0.0.1:8081 --wisp-path /wisp; \
    read'"

tmux new-window -t gromnie-dev -n vite \
  "cd crates/gromnie-web && npm run dev; read"

tmux attach -t gromnie-dev

# In Chrome: http://localhost:5173/demo/
```

### Docker Compose

```bash
docker compose up --build -d
# Web: http://localhost:8080/
# Proxy: port 8081 (internal, proxied through nginx)
```

### Local dev (manual, without tmux)

```bash
# Terminal 1: build + run proxy
cargo xtask web build
cargo build -p gromnie-proxy
bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
  ./target/debug/gromnie-proxy --listen 127.0.0.1:8081 --wisp-path /wisp'

# Terminal 2: Vite dev server
cd crates/gromnie-web && npm run dev

# Open: http://localhost:5173/demo/
```
