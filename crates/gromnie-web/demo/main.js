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
const selectCharBtn = document.getElementById("select-char");

let wasm = null;
let client = null;
let characters = [];

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
    const target = document.getElementById(`log-${tab.dataset.tab}`);
    if (target) target.classList.add("active");
  });
});

async function loadWasm() {
  try {
    wasm = await import("./pkg/gromnie_web.js");
    await wasm.default();
    log("wasm loaded from ../pkg/gromnie_web.js");
    loginBtn.disabled = false;
  } catch (err) {
    loginBtn.disabled = true;
    log("missing wasm-bindgen pkg output. generate it with:");
    log("  cargo xtask web build");
    log(`loader error: ${err?.message ?? String(err)}`);
  }
}

function handleEvent(eventDesc) {
  logEvent(eventDesc);

  // Parse character list events
  if (eventDesc.includes("CharacterListReceived")) {
    const charMatch = eventDesc.match(/characters:\s*\[(.*?)\]/s);
    if (charMatch) {
      parseCharacterList(eventDesc);
    }
  }
}

function handleNetLog(entry) {
  const color = entry.startsWith("[TX]") ? "#f88" : "#8f8";
  const line = document.createElement("div");
  line.style.color = color;
  line.textContent = entry;
  netLogEl.appendChild(line);
  // Keep last 200 lines
  while (netLogEl.children.length > 200) {
    netLogEl.removeChild(netLogEl.firstChild);
  }
  netLogEl.scrollTop = netLogEl.scrollHeight;
}

function parseCharacterList(eventDesc) {
  // Match "CharacterIdentity { character_id: ObjectId(N), name: \"Foo\", ... }"
  const charRegex = /CharacterIdentity\s*\{\s*character_id:\s*ObjectId\((\d+)\),\s*name:\s*"([^"]+)"/g;
  characters = [];
  let match;
  while ((match = charRegex.exec(eventDesc)) !== null) {
    characters.push({ id: parseInt(match[1]), name: match[2] });
  }

  if (characters.length > 0) {
    charListEl.textContent = characters
      .map((c, i) => `${i + 1}. ${c.name} (ID: ${c.id})`)
      .join("\n");
    selectCharBtn.disabled = false;
    log(`found ${characters.length} character(s)`);
  } else {
    charListEl.textContent = "(parse failed — check event log)";
  }
}

async function doLogin() {
  try {
    if (!wasm) {
      throw new Error("wasm module not loaded");
    }

    loginBtn.disabled = true;

    const wsProto = location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${wsProto}//${location.host}/wisp/`;
    log(`connecting to ${wsUrl}...`);

    client = new wasm.WasmClient();

    // Set up event callback
    client.set_on_event(handleEvent);
    client.set_on_net_log(handleNetLog);

    await client.connect(
      wsUrl,
      hostEl.value.trim(),
      parseInt(portEl.value.trim(), 10),
      accountEl.value.trim(),
      passwordEl.value.trim()
    );

    log("connected and login sent, waiting for server response...");
  } catch (err) {
    log(`login error: ${err?.message ?? String(err)}`);
    loginBtn.disabled = false;
  }
}

async function doSelectCharacter() {
  try {
    if (!client) {
      throw new Error("login first");
    }
    if (characters.length === 0) {
      throw new Error("no characters available");
    }

    const char = characters[0]; // Select first character
    log(`selecting character: ${char.name} (ID: ${char.id})...`);
    client.select_character(char.id, accountEl.value.trim());
    log("character selected, entering world...");
  } catch (err) {
    log(`select error: ${err?.message ?? String(err)}`);
  }
}

loginBtn.addEventListener("click", doLogin);
selectCharBtn.addEventListener("click", doSelectCharacter);

loadWasm();
