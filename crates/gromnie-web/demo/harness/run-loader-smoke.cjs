#!/usr/bin/env node
const http = require("node:http");
const fs = require("node:fs");
const fsp = require("node:fs/promises");
const path = require("node:path");
const os = require("node:os");
const { spawn } = require("node:child_process");

const projectRoot = path.resolve(__dirname, "..", "..");
const host = "127.0.0.1";
const port = 8787;
const args = process.argv.slice(2);
const expectMode = args.includes("--expect-missing") ? "missing" : "loaded";
const scenarioArg = args.find((x) => x.startsWith("--scenario="));
const scenario = scenarioArg ? scenarioArg.slice("--scenario=".length) : "loader";
const baseUrl =
  scenario === "connect-flow"
    ? `http://${host}:${port}/demo/?harness=connect-flow`
    : `http://${host}:${port}/demo/`;

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".mjs": "application/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".d.ts": "text/plain; charset=utf-8",
  ".json": "application/json; charset=utf-8",
};

function safeJoin(root, urlPath) {
  const normalized = path
    .normalize(urlPath)
    .replace(/^(\.\.[/\\])+/, "")
    .replace(/^[/\\]+/, "");
  return path.join(root, normalized);
}

function staticServer(rootDir) {
  return http.createServer(async (req, res) => {
    try {
      const reqUrl = new URL(req.url || "/", `http://${host}:${port}`);
      let pathname = decodeURIComponent(reqUrl.pathname);
      if (pathname.endsWith("/")) pathname += "index.html";
      const filePath = safeJoin(rootDir, pathname);
      if (!filePath.startsWith(rootDir)) {
        res.writeHead(403).end("Forbidden");
        return;
      }
      const stat = await fsp.stat(filePath).catch(() => null);
      if (!stat || !stat.isFile()) {
        res.writeHead(404).end("Not found");
        return;
      }
      const ext = path.extname(filePath);
      const contentType = MIME[ext] || "application/octet-stream";
      res.writeHead(200, { "Content-Type": contentType });
      fs.createReadStream(filePath).pipe(res);
    } catch (err) {
      res.writeHead(500).end(String(err));
    }
  });
}

function findChromeExecutable() {
  const envCandidate = process.env.CHROME_BIN;
  if (envCandidate && fs.existsSync(envCandidate)) {
    return envCandidate;
  }

  const fallbackCandidates = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  ];

  for (const candidate of fallbackCandidates) {
    if (fs.existsSync(candidate)) return candidate;
  }

  const playwrightCache = path.join(os.homedir(), "Library/Caches/ms-playwright");
  if (fs.existsSync(playwrightCache)) {
    const versioned = fs
      .readdirSync(playwrightCache, { withFileTypes: true })
      .filter((d) => d.isDirectory() && d.name.startsWith("chromium-"))
      .map((d) => path.join(playwrightCache, d.name))
      .sort()
      .reverse();

    for (const dir of versioned) {
      const candidate = path.join(
        dir,
        "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
      );
      if (fs.existsSync(candidate)) return candidate;
    }
  }

  return null;
}

function decodeHtml(text) {
  return text
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'");
}

function runChromeDump(chromePath) {
  return new Promise((resolve, reject) => {
    const args = [
      "--headless=new",
      "--disable-gpu",
      "--virtual-time-budget=5000",
      "--dump-dom",
      baseUrl,
    ];
    const child = spawn(chromePath, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (d) => (stdout += d.toString()));
    child.stderr.on("data", (d) => (stderr += d.toString()));
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(new Error(`chrome exited with code ${code}\n${stderr}`));
      }
    });
  });
}

async function main() {
  const chromePath = findChromeExecutable();
  if (!chromePath) {
    throw new Error(
      "Could not find a Chrome/Chromium executable. Set CHROME_BIN or install via: npx playwright install chromium",
    );
  }

  const server = staticServer(projectRoot);
  await new Promise((resolve) => server.listen(port, host, resolve));
  console.log(`HARNESS: serving ${projectRoot} at ${baseUrl}`);
  console.log(`HARNESS: using browser ${chromePath}`);

  try {
    const { stdout, stderr } = await runChromeDump(chromePath);
    const preMatch = stdout.match(/<pre id="log"[^>]*>([\s\S]*?)<\/pre>/i);
    const logText = preMatch ? decodeHtml(preMatch[1]).trim() : "";

    console.log("\n=== DEMO LOG OUTPUT ===");
    console.log(logText || "(no log output found in #log)");

    if (stderr.trim()) {
      console.log("\n=== BROWSER STDERR ===");
      console.log(stderr.trim());
    }

    let pass = false;
    if (scenario === "loader") {
      pass =
        expectMode === "loaded"
          ? logText.includes("wasm loaded from ../pkg/gromnie_web.js")
          : logText.includes("missing wasm-bindgen pkg output");
    } else if (scenario === "connect-flow") {
      const tcpOpenError =
        logText.includes("open_tcp_stream error: connect first") ||
        logText.includes("open_tcp_stream error: not connected");
      const udpOpenError =
        logText.includes("open_udp_stream error: connect first") ||
        logText.includes("open_udp_stream error: not connected");
      pass =
        logText.includes("wasm loaded from ../pkg/gromnie_web.js") &&
        logText.includes("connect error:") &&
        tcpOpenError &&
        udpOpenError;
    } else {
      throw new Error(`Unknown scenario "${scenario}"`);
    }

    if (!pass) {
      throw new Error(
        `Harness assertion failed for scenario "${scenario}" and expected mode "${expectMode}".`,
      );
    }

    console.log(`\nHARNESS PASS: scenario "${scenario}" matched expected output`);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

main().catch((err) => {
  console.error(`HARNESS FAIL: ${err.message}`);
  process.exit(1);
});
