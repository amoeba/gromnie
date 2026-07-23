// SharedWorker that holds GromnieClient and manages the WISP connection.
// Persists across page reloads so the game connection survives HMR and full reloads.

// Persistent state (survives as long as the worker lives)
let GromnieClient = null;
let client = null;
let characters = [];
let selectedCharId = null;
let inWorld = false;
let wispUrl = null;
const ports = [];

// Worker instance ID — created once at load time. If it changes across page
// loads, the worker was recreated. If it stays the same, it persisted.
const _id = Math.random().toString(36).slice(2, 8);

function broadcast(msg) {
  for (const port of ports) {
    port.postMessage(msg);
  }
}

function getState() {
  return {
    type: "state",
    connected: !!client,
    inWorld,
    characters,
    selectedCharId,
    charName: client
      ? characters.find((c) => c.id === selectedCharId)?.name ?? null
      : null,
  };
}

function handleEvent(eventDesc) {
  // Track state from events for reconnection
  if (eventDesc.includes("CharacterListReceived")) {
    const charRegex =
      /CharacterIdentity\s*\{\s*character_id:\s*ObjectId\((\d+)\),\s*name:\s*"([^"]+)"/g;
    characters = [];
    let match;
    while ((match = charRegex.exec(eventDesc)) !== null) {
      characters.push({ id: parseInt(match[1]), name: match[2] });
    }
  }

  if (
    eventDesc.includes("Disconnected") ||
    eventDesc.includes("AuthenticationFailed")
  ) {
    client = null;
    characters = [];
    selectedCharId = null;
    inWorld = false;
  }

  if (eventDesc.includes("CharacterError")) {
    const msgMatch = eventDesc.match(/error_message:\s*"([^"]*)"/);
    if (msgMatch && msgMatch[1] === "Logon") {
      client = null;
      inWorld = false;
    }
  }

  // Forward to all connected UI pages
  broadcast({ type: "event", data: eventDesc });
}

function handleNetLog(entry) {
  broadcast({ type: "net_log", data: entry });
}

async function doConnect(host, port, account, password) {
  if (!GromnieClient) throw new Error("wasm module not loaded");
  if (client) throw new Error("already connected");

  if (!wispUrl) {
    wispUrl = `ws://${self.location.host}/wisp`;
  }

  client = new GromnieClient(wispUrl);
  client.set_on_event(handleEvent);
  client.set_on_net_log(handleNetLog);

  await client.connect(host, port, account, password);
}

function doSelectCharacter(characterId) {
  if (!client) throw new Error("login first");
  client.select_character(characterId);
  selectedCharId = characterId;
  inWorld = true;
}

function doSendChat(message) {
  if (!client) throw new Error("not connected");
  client.send_chat(message);
}

async function doDisconnect() {
  if (!client) return;
  await client.disconnect();
  client = null;
  characters = [];
  selectedCharId = null;
  inWorld = false;
}

async function checkProxy() {
  if (!wispUrl) {
    wispUrl = `ws://${self.location.host}/wisp`;
  }
  try {
    const ws = new WebSocket(wispUrl);
    const ok = await new Promise((resolve) => {
      ws.onopen = () => {
        ws.close();
        resolve(true);
      };
      ws.onerror = () => resolve(false);
      setTimeout(() => {
        try {
          ws.close();
        } catch {}
        resolve(false);
      }, 3000);
    });
    broadcast({
      type: "proxy_status",
      status: ok ? "reachable" : "unreachable",
    });
  } catch {
    broadcast({ type: "proxy_status", status: "unreachable" });
  }
}

async function initWasm() {
  try {
    broadcast({ type: "wasm_status", status: "loading" });
    const mod = await import("./pkg/index.mjs");
    GromnieClient = mod.GromnieClient;
    broadcast({ type: "wasm_status", status: "ready" });
  } catch (err) {
    broadcast({
      type: "wasm_status",
      status: "error",
      error: err?.message ?? String(err),
    });
  }
}

// Listen for connections from main pages
self.onconnect = (e) => {
  const port = e.ports[0];
  ports.push(port);

  // Send worker ID so the page can tell if it's a new or reused worker
  port.postMessage({ type: "worker_id", id: _id });

  port.onmessage = async (msgEvent) => {
    const msg = msgEvent.data;
    try {
      switch (msg.type) {
        case "get_state":
          port.postMessage(getState());
          break;
        case "connect":
          await doConnect(
            msg.host,
            msg.port,
            msg.account,
            msg.password,
          );
          port.postMessage({ type: "connected" });
          break;
        case "select_character": {
          doSelectCharacter(msg.characterId);
          const charName =
            characters.find((c) => c.id === msg.characterId)?.name ?? "";
          port.postMessage({ type: "character_selected", charName });
          break;
        }
        case "send_chat":
          doSendChat(msg.message);
          port.postMessage({ type: "chat_sent" });
          break;
        case "disconnect":
          await doDisconnect();
          port.postMessage({ type: "disconnected" });
          break;
        default:
          port.postMessage({
            type: "error",
            message: `unknown message type: ${msg.type}`,
          });
      }
    } catch (err) {
      port.postMessage({ type: "error", message: err?.message ?? String(err) });
    }
  };

  port.start();
};

// Initialize WASM when worker loads
initWasm().then(() => checkProxy());
