// <gromnie-world-view> — "The Realm" — Chat messages, input, exit with Ultima framing.

const CHAT_COLORS = {
  0x00: "var(--chat-default, #c8b07a)",
  0x03: "var(--chat-say, #4aa8a8)",
  0x04: "var(--chat-tell, #4a9a4a)",
  0x05: "var(--chat-shout, #c48a3a)",
  0x06: "var(--chat-system, #c44a3a)",
  0x07: "var(--chat-emote, #a44aa8)",
  0x11: "var(--chat-guild, #4a6ac4)",
};

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    .realm-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: var(--sp-3, 0.75rem);
      padding-bottom: var(--sp-2, 0.5rem);
      border-bottom: 1px solid var(--codex-border, #3d2e1a);
    }
    .realm-header h2 {
      font-family: var(--font-display, serif);
      color: var(--codex-heading, #d4a843);
      font-size: 1.2rem;
      margin: 0;
    }
    #char-name {
      font-family: var(--font-display, serif);
      color: var(--codex-gold, #d4a843);
      font-size: 0.9rem;
    }
    #char-name::before {
      content: '» ';
      color: var(--codex-border, #3d2e1a);
    }
    #chat-messages {
      border: 1px solid var(--codex-border, #3d2e1a);
      background: var(--codex-bg, #0c0a08);
      padding: var(--sp-2, 0.5rem);
      height: calc(20 * 1.4em + 1rem);
      width: calc(80 * 0.62ch + 1rem);
      max-width: 100%;
      overflow-y: auto;
      white-space: pre-wrap;
      word-wrap: break-word;
      font-size: 0.75rem;
      line-height: 1.4;
      margin-bottom: var(--sp-2, 0.5rem);
    }
    .chat-line {
      margin-bottom: 1px;
    }
    .chat-line--player {
      color: var(--chat-tell, #4a9a4a);
    }
    .chat-line--player::before {
      content: '» ';
      color: var(--codex-border, #3d2e1a);
    }
    .chat-input-row {
      display: flex;
      gap: var(--sp-1, 0.25rem);
      width: calc(80 * 0.62ch + 1rem);
      max-width: 100%;
    }
    #chat-input {
      flex: 1;
      background: var(--codex-panel, #14110d);
      border: 1px solid var(--codex-border, #3d2e1a);
      color: var(--codex-text-bright, #e8d4a0);
      font-family: var(--font-body, monospace);
      font-size: 0.85rem;
      padding: var(--sp-1, 0.25rem) var(--sp-2, 0.5rem);
      border-radius: 2px;
    }
    #chat-input:focus {
      outline: none;
      border-color: var(--codex-gold-dim, #8a6e2a);
      box-shadow: 0 0 8px rgba(212, 168, 67, 0.15);
    }
    #chat-input::placeholder {
      color: var(--codex-text-dim, #7a6840);
      font-style: italic;
    }
    .actions {
      display: flex;
      gap: var(--sp-2, 0.5rem);
      margin-top: var(--sp-2, 0.5rem);
    }
  </style>

  <div class="realm-header">
    <h2>⚜ The Realm ⚜</h2>
    <span id="char-name"></span>
  </div>

  <div id="chat-messages"></div>

  <div class="chat-input-row">
    <input id="chat-input" placeholder="Speak unto the realm..." />
    <button class="codex-btn" id="chat-send">Send</button>
  </div>

  <div class="actions">
    <button class="codex-btn codex-btn--danger" id="exit-world" disabled>
      ↺ Depart the Realm
    </button>
  </div>
`;

class GromnieWorldView extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));

    this._charNameEl = this.shadowRoot.getElementById("char-name");
    this._chatMessagesEl = this.shadowRoot.getElementById("chat-messages");
    this._chatInputEl = this.shadowRoot.getElementById("chat-input");
    this._chatSendBtn = this.shadowRoot.getElementById("chat-send");
    this._exitBtn = this.shadowRoot.getElementById("exit-world");

    this._chatSendBtn.addEventListener("click", () => this._doSendChat());
    this._chatInputEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        this._doSendChat();
      }
    });
    this._exitBtn.addEventListener("click", () => {
      this.dispatchEvent(
        new CustomEvent("gromnie:exit-world", {
          bubbles: true,
          composed: true,
        })
      );
    });
  }

  setCharName(name) {
    this._charNameEl.textContent = name;
  }

  setExitEnabled(enabled) {
    this._exitBtn.disabled = !enabled;
  }

  clearChat() {
    this._chatMessagesEl.innerHTML = "";
  }

  appendChat(text, msgType = 0) {
    const div = document.createElement("div");
    div.className = "chat-line";
    div.textContent = text;
    div.style.color = CHAT_COLORS[msgType] || "var(--chat-default, #c8b07a)";
    this._chatMessagesEl.appendChild(div);
    this._chatMessagesEl.scrollTop = this._chatMessagesEl.scrollHeight;
  }

  focusInput() {
    this._chatInputEl.focus();
  }

  _doSendChat() {
    const msg = this._chatInputEl.value.trim();
    if (!msg) return;
    this.dispatchEvent(
      new CustomEvent("gromnie:send-chat", {
        detail: { message: msg },
        bubbles: true,
        composed: true,
      })
    );
    this.appendChat(`> ${msg}`, 0x04);
    this._chatInputEl.value = "";
  }
}

export { GromnieWorldView };
