const logEls = {
  all: document.getElementById("log-all"),
  game: document.getElementById("log-game"),
  protocol: document.getElementById("log-protocol"),
  state: document.getElementById("log-state"),
  system: document.getElementById("log-system"),
};
const netLogEl = document.getElementById("net-log");
const hostEl = document.getElementById("host");
const portEl = document.getElementById("port");
const accountEl = document.getElementById("account");
const passwordEl = document.getElementById("password");
const charListEl = document.getElementById("char-list");

const loginBtn = document.getElementById("login");
const enterWorldBtn = document.getElementById("enter-world");
const exitWorldBtn = document.getElementById("exit-world");
const resetFormBtn = document.getElementById("reset-form");

const accountView = document.getElementById("account-view");
const characterView = document.getElementById("character-view");
const worldView = document.getElementById("world-view");

const chatMessagesEl = document.getElementById("chat-messages");
const chatInputEl = document.getElementById("chat-input");
const chatSendBtn = document.getElementById("chat-send");
const worldCharNameEl = document.getElementById("world-char-name");

const wasmStatusEl = document.getElementById("wasm-status");
const proxyStatusEl = document.getElementById("proxy-status");

const errorOverlayEl = document.getElementById("error-overlay");
const errorMessageEl = document.getElementById("error-message");
const errorOkBtn = document.getElementById("error-ok");

const STORAGE_KEY = "gromnie-form";

let GromnieClient = null;
let client = null;
let characters = [];
let selectedCharId = null;
let inWorld = false;

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

[hostEl, portEl, accountEl].forEach((el) =>
  el.addEventListener("input", saveForm)
);

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

// Tab switching
document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".tab-content").forEach((c) => c.classList.remove("active"));
    tab.classList.add("active");
    const targetId = tab.dataset.tab === "network" ? "net-log" : `log-${tab.dataset.tab}`;
    const target = document.getElementById(targetId);
    if (target) target.classList.add("active");
  });
});

const CHAT_COLORS = {
  0x00: "#000",    // Broadcast
  0x03: "#0cc",    // Tell (incoming)
  0x04: "#0a0",    // OutgoingTell
  0x05: "#a80",    // System
  0x06: "#c00",    // Combat
  0x07: "#c0c",    // Magic
  0x11: "#00f",    // Overland channel
};

function appendChatLine(text, msgType = 0) {
  const div = document.createElement("div");
  div.textContent = text;
  div.style.color = CHAT_COLORS[msgType] || "#000";
  chatMessagesEl.appendChild(div);
  chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
}

function handleEvent(eventDesc) {
  // Skip noisy events when not in world
  if (!inWorld && !eventDesc.includes("Disconnected") && !eventDesc.includes("AuthenticationFailed") && !eventDesc.includes("CharacterError") && !eventDesc.includes("CharacterListReceived")) {
    return;
  }

  logEvent(eventDesc);

  // Re-enable login button on disconnect or error
  if (eventDesc.includes("Disconnected") || eventDesc.includes("AuthenticationFailed")) {
    if (eventDesc.includes("AuthenticationFailed")) {
      showError("Authentication failed. Please check your credentials.");
    }
    loginBtn.disabled = false;
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = true;
    client = null;
    characters = [];
    selectedCharId = null;
    if (inWorld) {
      inWorld = false;
      showView(accountView);
    }
  }

  // Handle character error
  if (eventDesc.includes("CharacterError")) {
    const msgMatch = eventDesc.match(/error_message:\s*"([^"]*)"/);
    const errMsg = msgMatch ? msgMatch[1] : "unknown";
    log(`character error: ${errMsg}`);
    enterWorldBtn.disabled = true;

    if (errMsg === "Logon") {
      showError("Login failed. The server rejected the logon request.");
      client = null;
      inWorld = false;
    } else if (inWorld) {
      inWorld = false;
      showView(characterView);
    }
  }

  // Parse character list events
  if (eventDesc.includes("CharacterListReceived")) {
    parseCharacterList(eventDesc);
  }

  // Parse chat messages
  if (eventDesc.includes("ChatMessageReceived")) {
    const msgMatch = eventDesc.match(/message:\s*"((?:[^"\\]|\\.)*)"/);
    const typeMatch = eventDesc.match(/message_type:\s*(\d+)/);
    if (msgMatch) {
      const text = msgMatch[1].replace(/\\n/g, "\n").replace(/\\"/g, '"').replace(/\\\\/g, '\\');
      const msgType = typeMatch ? parseInt(typeMatch[1]) : 0;
      text.split("\n").forEach(line => appendChatLine(line, msgType));
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

function parseCharacterList(eventDesc) {
  const charRegex = /CharacterIdentity\s*\{\s*character_id:\s*ObjectId\((\d+)\),\s*name:\s*"([^"]+)"/g;
  characters = [];
  let match;
  while ((match = charRegex.exec(eventDesc)) !== null) {
    characters.push({ id: parseInt(match[1]), name: match[2] });
  }

  if (characters.length > 0) {
    charListEl.innerHTML = "";
    characters.forEach((c, i) => {
      const row = document.createElement("div");
      row.textContent = c.name;
      row.style.cssText = "padding: 0.3rem 0.5rem; cursor: pointer; border: 1px solid transparent; border-radius: 3px; margin-bottom: 0.15rem;";
      row.addEventListener("click", () => {
        charListEl.querySelectorAll("div").forEach(r => r.style.background = "");
        row.style.background = "#dde";
        selectedCharId = c.id;
        enterWorldBtn.disabled = false;
      });
      if (i === 0) {
        row.style.background = "#dde";
        selectedCharId = c.id;
        enterWorldBtn.disabled = false;
      }
      charListEl.appendChild(row);
    });
    log(`found ${characters.length} character(s)`);
    showView(characterView);
  } else {
    charListEl.textContent = "(parse failed — check event log)";
  }
}

async function doLogin() {
  try {
    if (!GromnieClient) {
      throw new Error("wasm module not loaded");
    }
    if (client) {
      throw new Error("already connected");
    }

    loginBtn.disabled = true;
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = true;
    selectedCharId = null;

    const wsProto = location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${wsProto}//${location.host}/wisp/`;
    log(`connecting to ${wsUrl}...`);

    client = new GromnieClient(wsUrl);
    client.set_on_event(handleEvent);
    client.set_on_net_log(handleNetLog);

    const loginTimeout = setTimeout(() => {
      if (client) {
        log("login timed out — no server response");
        loginBtn.disabled = false;
        setStatus(proxyStatusEl, "timeout", false);
        client = null;
      }
    }, 10000);

    await client.connect(
      hostEl.value.trim(),
      parseInt(portEl.value.trim(), 10),
      accountEl.value.trim(),
      passwordEl.value.trim()
    );

    clearTimeout(loginTimeout);
    setStatus(proxyStatusEl, "connected", true);
    log("connected and login sent, waiting for server response...");
  } catch (err) {
    log(`login error: ${err?.message ?? String(err)}`);
    loginBtn.disabled = false;
    setStatus(proxyStatusEl, "error", false);
  }
}

async function doEnterWorld() {
  try {
    if (!client) throw new Error("login first");
    if (selectedCharId === null) throw new Error("select a character first");

    const char = characters.find((c) => c.id === selectedCharId);
    log(`entering world with character: ${char.name} (ID: ${char.id})...`);
    enterWorldBtn.disabled = true;
    exitWorldBtn.disabled = false;
    client.select_character(selectedCharId);
    log("character selected, entering world...");
    inWorld = true;
    chatMessagesEl.innerHTML = "";
    worldCharNameEl.textContent = char.name;
    showView(worldView);
    chatInputEl.focus();
  } catch (err) {
    log(`enter world error: ${err?.message ?? String(err)}`);
  }
}

async function doExitWorld() {
  try {
    if (!client) return;
    inWorld = false;
    enterWorldBtn.disabled = false;
    showView(characterView);
    log("exited world, back to character select");
  } catch (err) {
    log(`exit error: ${err?.message ?? String(err)}`);
  }
}

function doSendChat() {
  const msg = chatInputEl.value.trim();
  if (!msg || !client) return;
  try {
    client.send_chat(msg);
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

function showError(msg) {
  errorMessageEl.textContent = msg;
  errorOverlayEl.classList.add("active");
}

function hideError() {
  errorOverlayEl.classList.remove("active");
  showView(accountView);
  loginBtn.disabled = false;
}

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

async function loadWasm() {
  try {
    const mod = await import("./pkg/index.mjs");
    GromnieClient = mod.GromnieClient;
    log("wasm loaded from ./pkg/index.mjs");
    setStatus(wasmStatusEl, "ready", true);
    loginBtn.disabled = false;
  } catch (err) {
    loginBtn.disabled = true;
    setStatus(wasmStatusEl, "error", false);
    log("missing wasm-bindgen pkg output. generate it with:");
    log("  cargo xtask web build");
    log(`loader error: ${err?.message ?? String(err)}`);
  }
}

async function checkProxy() {
  const wsProto = location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${wsProto}//${location.host}/wisp/`;
  try {
    const ws = new WebSocket(wsUrl);
    const ok = await new Promise((resolve) => {
      ws.onopen = () => { ws.close(); resolve(true); };
      ws.onerror = () => resolve(false);
      setTimeout(() => { try { ws.close(); } catch {} resolve(false); }, 3000);
    });
    setStatus(proxyStatusEl, ok ? "reachable" : "unreachable", ok);
    log(`proxy ${wsUrl}: ${ok ? "reachable" : "unreachable"}`);
  } catch {
    setStatus(proxyStatusEl, "unreachable", false);
  }
}

loadForm();
loadWasm();
checkProxy();
