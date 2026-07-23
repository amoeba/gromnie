// <gromnie-world-view> — Chat messages, chat input, exit world button.

const CHAT_COLORS = {
  0x00: "#000",
  0x03: "#0cc",
  0x04: "#0a0",
  0x05: "#a80",
  0x06: "#c00",
  0x07: "#c0c",
  0x11: "#00f",
};

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    #char-name {
      font-weight: bold;
      margin-bottom: 0.3rem;
      color: #333;
    }
    #chat-messages {
      border: 1px solid #ccc;
      padding: 0.35rem;
      height: calc(20 * 1.2em + 0.7rem);
      width: calc(80 * 0.65ch + 0.7rem);
      overflow-y: auto;
      white-space: pre-wrap;
      font-size: 0.75rem;
      margin-bottom: 0.3rem;
    }
    .chat-input-row {
      display: flex;
      gap: 0.25rem;
      width: calc(80 * 0.65ch + 0.7rem);
    }
    #chat-input {
      flex: 1;
      padding: 0.2rem 0.3rem;
      font-size: 0.85rem;
      font-family: inherit;
    }
    button {
      padding: 0.25rem 0.5rem;
      font-size: 0.85rem;
      font-family: inherit;
    }
    .actions {
      display: flex;
      gap: 0.4rem;
      margin-top: 0.35rem;
    }
  </style>
  <div id="char-name"></div>
  <div id="chat-messages"></div>
  <div class="chat-input-row">
    <input id="chat-input" placeholder="Type a message..." />
    <button id="chat-send">Send</button>
  </div>
  <div class="actions">
    <button id="exit-world" disabled>Exit World</button>
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
    div.textContent = text;
    div.style.color = CHAT_COLORS[msgType] || "#000";
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
