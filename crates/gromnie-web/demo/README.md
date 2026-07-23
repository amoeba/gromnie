# gromnie-web demo

Tiny static browser demo for `GromnieClient`.

## 1) Generate wasm-bindgen output

From repo root:

```bash
cargo xtask web build
```

This should create `crates/gromnie-web/pkg/gromnie_web.js`.

## 2) Serve the demo directory

From repo root:

```bash
python3 -m http.server 8081 --directory crates/gromnie-web
```

Then open:

- <http://127.0.0.1:8081/demo/>

The demo imports `../pkg/index.mjs`, so it must be served from the `crates/gromnie-web` directory root.

## Harness: capture browser output in agent/CI logs

From repo root:

```bash
cargo xtask web build
node crates/gromnie-web/demo/harness/run-loader-smoke.cjs
```

This runs a headless browser against `demo/index.html`, captures the page log output (`#log`) and browser console/errors, and prints them to stdout.

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
