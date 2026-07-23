// UI logic — hot-reloadable module that handles all DOM interactions.
// This file is imported by main.js and can be HMR'd without killing the SharedWorker.
// v5

let port;
let logEls, netLogEl, hostEl, portEl, accountEl, passwordEl, charListEl;
let loginBtn, enterWorldBtn, exitWorldBtn, resetFormBtn;
let accountView, characterView, worldView;
let chatMessagesEl, chatInputEl, chatSendBtn, worldCharNameEl;
let wasmStatusEl, proxyStatusEl;
let errorOverlayEl, errorMessageEl, errorOkBtn;

const STORAGE_KEY = "gromnie-form";

let characters = [];
let selectedCharId = null;
let inWorld = false;
let currentLoginTimeout = null;

const CHAT_COLORS = {
  0x00: "#000",
  0x03: "#0cc",
  0x04: "#0a0",
  0x05: "#a80",
  0x06: "#c00",
  0x07: "#c0c",
  0x11: "#00f",
};

function setStatus(el, text, ok) {
  el.innerHTML = `<span class="label">${el.querySelector(".label").textContent}</span> ${text}`;
  el.style.borderColor = ok === true ? "#4a4" : ok === false ? "#c44" : "#ccc";
}

function saveForm() {
  const data = {
    host: hostEl.value,
    port: portEl.value,
    account: accountEl.value,
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
}

function loadForm() {
  try {
    const data = JSON.parse(localStorage.getItem(STORAGE_KEY));
    if (data) {
      if (data.host) hostEl.value = data.host;
      if (data.port) portEl.value = data.port;
      if (data.account) accountEl.value = data.account;
    }
  } catch {}
}

function showView(view) {
  accountView.classList.remove("active");
  characterView.classList.remove("active");
  worldView.classList.remove("active");
  view.classList.add("active");
}

function appendLog(pre, message) {
  const line = `[${new Date().toISOString()}] ${message}`;
  pre.textContent += `${line}\n`;
  pre.scrollTop = pre.scrollHeight;
}

function log(message) {
  appendLog(logEls.all, message);
}

function logEvent(eventDesc) {
  appendLog(logEls.all, `event: ${eventDesc}`);

  const typeMatch = eventDesc.match(/^(\w+):/);
  if (typeMatch) {
    const type = typeMatch[1].toLowerCase();
    if (logEls[type]) {
      appendLog(logEls[type], eventDesc);
    }
  }
}

function appendChatLine(text, msgType = 0) {
  const div = document.createElement("div");
  div.textContent = text;
  div.style.color = CHAT_COLORS[msgType] || "#000";
  chatMessagesEl.appendChild(div);
  chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
}

function renderCharacterList() {
  charListEl.innerHTML = "";
  characters.forEach((c, i) => {
    const row = document.createElement("div");
    row.textContent = c.name;
    row.style.cssText =
      "padding: 0.3rem 0.5rem; cursor: pointer; border: 1px solid transparent; border-radius: 3px; margin-bottom: 0.15rem;";
    row.addEventListener("click", () => {
      charListEl
        .querySelectorAll("div")
        .forEach((r) => (r.style.background = ""));
      row.style.background = "#dde";
      selectedCharId = c.id;
      enterWorldBtn.disabled = false;
    });
    if (i === 0 || c.id === selectedCharId) {
      row.style.background = "#dde";
      selectedCharId = c.id;
      enterWorldBtn.disabled = false;
    }
    charListEl.appendChild(row);
  });
}

function showError(msg) {
  errorMessageEl.textContent = msg;
  errorOverlayEl.classList.add("active");
}

function hideError() {
  errorOverlayEl.classList.remove("active");
  showView(accountView);
  loginBtn.disabled = false;
}

function restoreState(state) {
  if (!state.connected) {
    log("worker: not connected");
    return;
  }

  setStatus(proxyStatusEl, "connected", true);
  log("worker: reconnected to existing session");

  if (state.inWorld && state.charName) {
    inWorld = true;
    worldCharNameEl.textContent = state.charName;
    showView(worldView);
  } else if (state.characters.length > 0) {
    characters = state.characters;
    selectedCharId = state.selectedCharId;
    renderCharacterList();
    showView(characterView);
  }
}

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

  logEvent(eventDesc);

  if (
    eventDesc.includes("Disconnected") ||
    eventDesc.includes("AuthenticationFailed")
  ) {
    if (eventDesc.includes("AuthenticationFailed")) {
      showError("Authentication failed. Please check your credentials.");
    }
    loginBtn.disabled = false;
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = true;
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
    log(`character error: ${errMsg}`);
    enterWorldBtn.disabled = true;

    if (errMsg === "Logon") {
      showError("Login failed. The server rejected the logon request.");
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
      renderCharacterList();
      log(`found ${characters.length} character(s)`);
      showView(characterView);
    } else {
      charListEl.textContent = "(parse failed — check event log)";
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
      text.split("\n").forEach((line) => appendChatLine(line, msgType));
    }
  }
}

function handleNetLog(entry) {
  const color = entry.startsWith("[TX]") ? "#f88" : "#8f8";
  const line = document.createElement("div");
  line.style.color = color;
  line.textContent = entry;
  netLogEl.appendChild(line);
  while (netLogEl.children.length > 200) {
    netLogEl.removeChild(netLogEl.firstChild);
  }
  netLogEl.scrollTop = netLogEl.scrollHeight;
}

function send(msg) {
  port.postMessage(msg);
}

async function doLogin() {
  try {
    if (currentLoginTimeout) {
      clearTimeout(currentLoginTimeout);
      currentLoginTimeout = null;
    }

    loginBtn.disabled = true;
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = true;
    selectedCharId = null;

    log(
      `connecting to ${hostEl.value.trim()}:${portEl.value.trim()}...`,
    );

    send({
      type: "connect",
      host: hostEl.value.trim(),
      port: parseInt(portEl.value.trim(), 10),
      account: accountEl.value.trim(),
      password: passwordEl.value.trim(),
    });

    currentLoginTimeout = setTimeout(() => {
      currentLoginTimeout = null;
      log("login timed out — no server response");
      loginBtn.disabled = false;
      setStatus(proxyStatusEl, "timeout", false);
    }, 10000);
  } catch (err) {
    log(`login error: ${err?.message ?? String(err)}`);
    loginBtn.disabled = false;
    setStatus(proxyStatusEl, "error", false);
  }
}

async function doEnterWorld() {
  try {
    if (selectedCharId === null) throw new Error("select a character first");

    const char = characters.find((c) => c.id === selectedCharId);
    log(`entering world with character: ${char.name} (ID: ${char.id})...`);
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = false;
    send({ type: "select_character", characterId: selectedCharId });
    log("character selected, entering world...");
  } catch (err) {
    log(`enter world error: ${err?.message ?? String(err)}`);
  }
}

function doExitWorld() {
  inWorld = false;
  enterWorldBtn.disabled = false;
  showView(characterView);
  log("exited world, back to character select");
}

function doSendChat() {
  const msg = chatInputEl.value.trim();
  if (!msg) return;
  try {
    send({ type: "send_chat", message: msg });
    appendChatLine(`> ${msg}`, 0x04);
    chatInputEl.value = "";
  } catch (err) {
    log(`chat send error: ${err?.message ?? String(err)}`);
  }
}

function doResetForm() {
  hostEl.value = "play.coldeve.ac";
  portEl.value = "9000";
  accountEl.value = "";
  passwordEl.value = "";
  saveForm();
}

export function handleMessage(msg) {
  switch (msg.type) {
    case "worker_id":
      break;

    case "wasm_status":
      setStatus(wasmStatusEl, msg.status, msg.status === "ready");
      if (msg.status === "error") {
        loginBtn.disabled = true;
        log("missing wasm-bindgen pkg output. generate it with:");
        log("  cargo xtask web build");
        log(`loader error: ${msg.error}`);
      }
      if (msg.status === "ready") {
        loginBtn.disabled = false;
        log("wasm loaded from ./pkg/index.mjs");
      }
      break;

    case "proxy_status":
      setStatus(
        proxyStatusEl,
        msg.status,
        msg.status === "reachable",
      );
      log(`proxy: ${msg.status}`);
      break;

    case "state":
      restoreState(msg);
      break;

    case "connected":
      if (currentLoginTimeout) {
        clearTimeout(currentLoginTimeout);
        currentLoginTimeout = null;
      }
      setStatus(proxyStatusEl, "connected", true);
      log("connected and login sent, waiting for server response...");
      break;

    case "character_selected":
      inWorld = true;
      chatMessagesEl.innerHTML = "";
      worldCharNameEl.textContent = msg.charName;
      showView(worldView);
      chatInputEl.focus();
      break;

    case "chat_sent":
      break;

    case "disconnected":
      loginBtn.disabled = false;
      enterWorldBtn.disabled = true;
      exitWorldBtn.disabled = true;
      characters = [];
      selectedCharId = null;
      inWorld = false;
      showView(accountView);
      log("disconnected");
      break;

    case "event":
      handleEvent(msg.data);
      break;

    case "net_log":
      handleNetLog(msg.data);
      break;

    case "error":
      if (currentLoginTimeout) {
        clearTimeout(currentLoginTimeout);
        currentLoginTimeout = null;
      }
      log(`worker error: ${msg.message}`);
      break;
  }
}

export function init(portRef) {
  port = portRef;

  logEls = {
    all: document.getElementById("log-all"),
    game: document.getElementById("log-game"),
    protocol: document.getElementById("log-protocol"),
    state: document.getElementById("log-state"),
    system: document.getElementById("log-system"),
  };
  netLogEl = document.getElementById("net-log");
  hostEl = document.getElementById("host");
  portEl = document.getElementById("port");
  accountEl = document.getElementById("account");
  passwordEl = document.getElementById("password");
  charListEl = document.getElementById("char-list");

  loginBtn = document.getElementById("login");
  enterWorldBtn = document.getElementById("enter-world");
  exitWorldBtn = document.getElementById("exit-world");
  resetFormBtn = document.getElementById("reset-form");

  accountView = document.getElementById("account-view");
  characterView = document.getElementById("character-view");
  worldView = document.getElementById("world-view");

  chatMessagesEl = document.getElementById("chat-messages");
  chatInputEl = document.getElementById("chat-input");
  chatSendBtn = document.getElementById("chat-send");
  worldCharNameEl = document.getElementById("world-char-name");

  wasmStatusEl = document.getElementById("wasm-status");
  proxyStatusEl = document.getElementById("proxy-status");

  errorOverlayEl = document.getElementById("error-overlay");
  errorMessageEl = document.getElementById("error-message");
  errorOkBtn = document.getElementById("error-ok");

  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document
        .querySelectorAll(".tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".tab-content")
        .forEach((c) => c.classList.remove("active"));
      tab.classList.add("active");
      const targetId =
        tab.dataset.tab === "network" ? "net-log" : `log-${tab.dataset.tab}`;
      const target = document.getElementById(targetId);
      if (target) target.classList.add("active");
    });
  });

  [hostEl, portEl, accountEl].forEach((el) =>
    el.addEventListener("input", saveForm),
  );

  errorOkBtn.addEventListener("click", hideError);
  resetFormBtn.addEventListener("click", doResetForm);
  loginBtn.addEventListener("click", doLogin);
  enterWorldBtn.addEventListener("click", doEnterWorld);
  exitWorldBtn.addEventListener("click", doExitWorld);
  chatSendBtn.addEventListener("click", doSendChat);
  chatInputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      doSendChat();
    }
  });

  loadForm();
  send({ type: "get_state" });
}
