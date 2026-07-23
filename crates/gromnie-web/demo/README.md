# gromnie-web demo

Tiny static browser demo for `GromnieClient`.

## Architecture

The UI is built with web components for HMR support:

- **`main.js`** — Stable entry point. Creates the SharedWorker and routes
  messages to components. **Never edit during UI development.**
- **`components/`** — Web components that encapsulate UI logic:
  - `status-bar.js` — WASM/Proxy/Build status
  - `login-form.js` — Account view
  - `character-select.js` — Character list
  - `world-view.js` — Chat interface
  - `log-viewer.js` — Tabbed log panels
  - `error-overlay.js` — Error modal
  - `index.js` — Barrel file

The SharedWorker (`public/worker.js`) holds the `GromnieClient` instance and
WISP connection, persisting across page reloads so the game connection survives
HMR and full page reloads.

## 1) Generate wasm-bindgen output

From repo root:

```bash
cargo xtask web build
```

This should create `crates/gromnie-web/pkg/gromnie_web.js`.

## 2) Install dependencies

```bash
cd crates/gromnie-web
npm install
```

## 3) Start the dev server

You need **two processes**: the WISP proxy and the Vite dev server.

### With tmux (recommended)

```bash
# From repo root
tmux new-session -d -s gromnie-dev -n proxy \
  "bash -c 'set -a && source .env && set +a && \
    cargo run -p gromnie-proxy -- --listen 127.0.0.1:8081 --wisp-path /wisp; \
    read'"

tmux new-window -t gromnie-dev -n vite \
  "cd crates/gromnie-web && npm run dev; read"

tmux attach -t gromnie-dev
```

### Manual (two terminals)

```bash
# Terminal 1: proxy
bash -c 'set -a && source crates/gromnie-web/.env && set +a && \
  cargo run -p gromnie-proxy -- --listen 127.0.0.1:8081 --wisp-path /wisp'

# Terminal 2: vite
cd crates/gromnie-web && npm run dev
```

Open: <http://localhost:5173/demo/>

## Harness: capture browser output in agent/CI logs

From repo root:

```bash
cargo xtask web build
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs
```

This runs a headless browser against `demo/index.html`, captures the page log
output (`#log`) and browser console/errors, and prints them to stdout.

If no Chrome/Chromium executable is found, install one with:

```bash
npx playwright install chromium
```

To assert the missing-pkg path instead:

```bash
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs --expect-missing
```

To run an automated interaction-flow check (connect + open TCP + open UDP):

```bash
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs --scenario=connect-flow
```
