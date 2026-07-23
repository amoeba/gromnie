// <gromnie-status-bar> — WASM, Proxy, and Build status with Ultima framing.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
      margin-bottom: var(--sp-3, 0.75rem);
    }
    .status-frame {
      border: 1px solid var(--codex-border, #3d2e1a);
      padding: 0.3rem 0.5rem;
      display: flex;
      gap: 1.5rem;
      align-items: center;
      background: var(--codex-panel, #14110d);
      font-size: 0.75rem;
    }
    .status-frame::before {
      content: '╠══ ';
      color: var(--codex-border, #3d2e1a);
      user-select: none;
    }
    .status-frame::after {
      content: ' ═╣';
      color: var(--codex-border, #3d2e1a);
      margin-left: auto;
      user-select: none;
    }
    .status-item {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      white-space: nowrap;
    }
    .status-label {
      color: var(--codex-text-dim, #7a6840);
      text-transform: uppercase;
      font-size: 0.65rem;
      letter-spacing: 0.08em;
    }
    .status-value {
      color: var(--codex-text, #c8b07a);
    }
    .dot {
      display: inline-block;
      width: 5px;
      height: 5px;
      border-radius: 50%;
      background: var(--codex-text-dim, #7a6840);
      flex-shrink: 0;
    }
    .dot--ok {
      background: var(--codex-green, #4a9a4a);
      box-shadow: 0 0 6px rgba(74, 154, 74, 0.4);
    }
    .dot--err {
      background: var(--codex-red, #c44a3a);
      box-shadow: 0 0 6px rgba(196, 74, 58, 0.4);
    }
    .dot--warn {
      background: var(--codex-orange, #c48a3a);
      box-shadow: 0 0 6px rgba(196, 138, 58, 0.4);
    }
    .sep {
      color: var(--codex-border, #3d2e1a);
      user-select: none;
    }
  </style>
  <div class="status-frame">
    <div class="status-item" id="wasm-status">
      <span class="dot"></span>
      <span class="status-label">Wasm</span>
      <span class="status-value">loading...</span>
    </div>
    <span class="sep">│</span>
    <div class="status-item" id="proxy-status">
      <span class="dot"></span>
      <span class="status-label">Proxy</span>
      <span class="status-value">--</span>
    </div>
    <span class="sep">│</span>
    <div class="status-item" id="version-status">
      <span class="dot dot--ok"></span>
      <span class="status-label">Build</span>
      <span class="status-value">__GIT_SHA__</span>
    </div>
  </div>
`;

class GromnieStatusBar extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));
    this._wasmEl = this.shadowRoot.getElementById("wasm-status");
    this._proxyEl = this.shadowRoot.getElementById("proxy-status");
  }

  setStatus(type, text, state) {
    const el = type === "wasm" ? this._wasmEl : this._proxyEl;
    const dot = el.querySelector(".dot");
    const value = el.querySelector(".status-value");

    dot.className = "dot";
    if (state === true || state === "ok") dot.classList.add("dot--ok");
    else if (state === "warn") dot.classList.add("dot--warn");
    else if (state === false || state === "err") dot.classList.add("dot--err");

    value.textContent = text;
  }
}

export { GromnieStatusBar };
