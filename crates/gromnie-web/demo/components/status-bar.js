// <gromnie-status-bar> — Displays WASM, Proxy, and Build status. (hmr v2)

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 0.5rem;
    }
    .status-item {
      padding: 0.3rem 0.6rem;
      border: 1px solid #ccc;
      border-radius: 4px;
      font-size: 0.75rem;
      background: #f8f8f8;
    }
    .status-item .label {
      color: #888;
    }
  </style>
  <div class="status-item" id="wasm-status">
    <span class="label">WASM</span> loading...
  </div>
  <div class="status-item" id="proxy-status">
    <span class="label">Proxy</span> --
  </div>
  <div class="status-item" id="version-status">
    <span class="label">Build</span> __GIT_SHA__
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

  setStatus(type, text, ok) {
    const el = type === "wasm" ? this._wasmEl : this._proxyEl;
    const label = el.querySelector(".label").textContent;
    el.innerHTML = `<span class="label">${label}</span> ${text}`;
    el.style.borderColor = ok === true ? "#4a4" : ok === false ? "#c44" : "#ccc";
  }
}

export { GromnieStatusBar };
