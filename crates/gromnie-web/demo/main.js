// Stable entry point — creates the SharedWorker and routes messages to components.
// This file should NEVER be edited during UI development.
// All UI logic lives in components/ and is hot-reloadable.

import "./components/index.js";

const worker = new SharedWorker("/worker.js");
const port = worker.port;
port.start();

// Get component references
const statusBar = document.querySelector("gromnie-status-bar");
const loginForm = document.querySelector("gromnie-login-form");
const charSelect = document.querySelector("gromnie-character-select");
const worldView = document.querySelector("gromnie-world-view");
const logViewer = document.querySelector("gromnie-log-viewer");
const errorOverlay = document.querySelector("gromnie-error-overlay");

// View containers (for show/hide)
const accountView = document.getElementById("account-view");
const characterView = document.getElementById("character-view");
const worldViewContainer = document.getElementById("world-view");

function showView(view) {
  accountView.classList.remove("active");
  characterView.classList.remove("active");
  worldViewContainer.classList.remove("active");
  view.classList.add("active");
}

// Listen for user actions from components
loginForm.addEventListener("gromnie:connect", (e) => {
  port.postMessage({ type: "connect", ...e.detail });
});

charSelect.addEventListener("gromnie:select-character", (e) => {
  port.postMessage({ type: "select_character", ...e.detail });
});

worldView.addEventListener("gromnie:send-chat", (e) => {
  port.postMessage({ type: "send_chat", ...e.detail });
});

worldView.addEventListener("gromnie:exit-world", () => {
  inWorld = false;
  charSelect.setEnterEnabled(true);
  showView(characterView);
  logViewer.log("exited world, back to character select");
});

errorOverlay.addEventListener("gromnie:dismiss-error", () => {
  loginForm.setLoginEnabled(true);
  showView(accountView);
});

// State
let characters = [];
let selectedCharId = null;
let inWorld = false;
let currentLoginTimeout = null;

// Event handling (from worker-forwarded events)
function handleEvent(eventDesc) {
  if (
    !inWorld &&
    !eventDesc.includes("Disconnected") &&
    !eventDesc.includes("AuthenticationFailed") &&
    !eventDesc.includes("CharacterError") &&
    !eventDesc.includes("CharacterListReceived")
  ) {
    return;
  }

  logViewer.logEvent(eventDesc);

  if (
    eventDesc.includes("Disconnected") ||
    eventDesc.includes("AuthenticationFailed")
  ) {
    if (eventDesc.includes("AuthenticationFailed")) {
      errorOverlay.show("Authentication failed. Please check your credentials.");
    }
    loginForm.setLoginEnabled(true);
    charSelect.setEnterEnabled(false);
    characters = [];
    selectedCharId = null;
    if (inWorld) {
      inWorld = false;
      showView(accountView);
    }
  }

  if (eventDesc.includes("CharacterError")) {
    const msgMatch = eventDesc.match(/error_message:\s*"([^"]*)"/);
    const errMsg = msgMatch ? msgMatch[1] : "unknown";
    logViewer.log(`character error: ${errMsg}`);
    charSelect.setEnterEnabled(false);

    if (errMsg === "Logon") {
      errorOverlay.show("Login failed. The server rejected the logon request.");
      inWorld = false;
    } else if (inWorld) {
      inWorld = false;
      showView(characterView);
    }
  }

  if (eventDesc.includes("CharacterListReceived")) {
    const charRegex =
      /CharacterIdentity\s*\{\s*character_id:\s*ObjectId\((\d+)\),\s*name:\s*"([^"]+)"/g;
    characters = [];
    let match;
    while ((match = charRegex.exec(eventDesc)) !== null) {
      characters.push({ id: parseInt(match[1]), name: match[2] });
    }

    if (characters.length > 0) {
      charSelect.setCharacters(characters);
      logViewer.log(`found ${characters.length} character(s)`);
      showView(characterView);
    }
  }

  if (eventDesc.includes("ChatMessageReceived")) {
    const msgMatch = eventDesc.match(
      /message:\s*"((?:[^"\\]|\\.)*)"/,
    );
    const typeMatch = eventDesc.match(/message_type:\s*(\d+)/);
    if (msgMatch) {
      const text = msgMatch[1]
        .replace(/\\n/g, "\n")
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, "\\");
      const msgType = typeMatch ? parseInt(typeMatch[1]) : 0;
      text.split("\n").forEach((line) => worldView.appendChat(line, msgType));
    }
  }
}

function restoreState(state) {
  if (!state.connected) {
    logViewer.log("worker: not connected");
    return;
  }

  statusBar.setStatus("proxy", "connected", true);
  logViewer.log("worker: reconnected to existing session");

  if (state.inWorld && state.charName) {
    inWorld = true;
    worldView.setCharName(state.charName);
    showView(worldViewContainer);
  } else if (state.characters.length > 0) {
    characters = state.characters;
    selectedCharId = state.selectedCharId;
    charSelect.setCharacters(characters);
    charSelect.setSelectedId(selectedCharId);
    showView(characterView);
  }
}

// Route worker messages to components
port.onmessage = (e) => {
  const msg = e.data;
  switch (msg.type) {
    case "worker_id":
      break;

    case "wasm_status":
      statusBar.setStatus("wasm", msg.status, msg.status === "ready");
      if (msg.status === "error") {
        loginForm.setLoginEnabled(false);
        logViewer.log("missing wasm-bindgen pkg output. generate it with:");
        logViewer.log("  cargo xtask web build");
        logViewer.log(`loader error: ${msg.error}`);
      }
      if (msg.status === "ready") {
        loginForm.setLoginEnabled(true);
        logViewer.log("wasm loaded from ./pkg/index.mjs");
      }
      break;

    // proxy_status is handled by checkProxy() in page context (not worker)
    // to avoid Cloudflare caching issues with SharedWorker fetch.

    case "state":
      restoreState(msg);
      break;

    case "connected":
      if (currentLoginTimeout) {
        clearTimeout(currentLoginTimeout);
        currentLoginTimeout = null;
      }
      statusBar.setStatus("proxy", "connected", true);
      logViewer.log("connected and login sent, waiting for server response...");
      break;

    case "character_selected":
      inWorld = true;
      worldView.clearChat();
      worldView.setCharName(msg.charName);
      worldView.setExitEnabled(true);
      showView(worldViewContainer);
      worldView.focusInput();
      break;

    case "chat_sent":
      break;

    case "disconnected":
      loginForm.setLoginEnabled(true);
      charSelect.setEnterEnabled(false);
      worldView.setExitEnabled(false);
      characters = [];
      selectedCharId = null;
      inWorld = false;
      showView(accountView);
      logViewer.log("disconnected");
      break;

    case "event":
      handleEvent(msg.data);
      break;

    case "net_log":
      logViewer.logNet(msg.data);
      break;

    case "error":
      if (currentLoginTimeout) {
        clearTimeout(currentLoginTimeout);
        currentLoginTimeout = null;
      }
      logViewer.log(`worker error: ${msg.message}`);
      break;
  }
};

// Proxy health check (runs in page context, not SharedWorker, to avoid
// Cloudflare caching issues with SharedWorker fetch).
async function checkProxy() {
  try {
    const resp = await fetch("/auth", { cache: "no-store", credentials: "include" });
    if (resp.ok) {
      statusBar.setStatus("proxy", "reachable", "ok");
      logViewer.log("proxy: reachable");
    } else if (resp.status === 401) {
      statusBar.setStatus("proxy", "auth required", "warn");
      logViewer.log("proxy: auth required");
    } else {
      statusBar.setStatus("proxy", "unreachable", "err");
      logViewer.log("proxy: unreachable");
    }
  } catch {
    statusBar.setStatus("proxy", "unreachable", "err");
    logViewer.log("proxy: unreachable");
  }
}

// Init: request state from worker, then check proxy
port.postMessage({ type: "get_state" });
checkProxy();
