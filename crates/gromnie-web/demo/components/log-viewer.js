// <gromnie-log-viewer> — "The Scrying Pool" — Tabbed log panels with Ultima framing.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
      border: 1px solid var(--codex-border, #3d2e1a);
      border-radius: 2px;
      background: var(--codex-panel, #14110d);
    }
    .scry-header {
      text-align: center;
      padding: var(--sp-2, 0.5rem) var(--sp-2, 0.5rem) 0;
      font-size: 0.7rem;
      color: var(--codex-text-dim, #7a6840);
      user-select: none;
    }
    .tabs {
      display: flex;
      gap: 0;
      padding: 0 var(--sp-2, 0.5rem);
      border-bottom: 1px solid var(--codex-border, #3d2e1a);
      overflow-x: auto;
    }
    .tab {
      padding: 0.3rem 0.7rem;
      cursor: pointer;
      font-size: 0.7rem;
      color: var(--codex-text-dim, #7a6840);
      border-bottom: 2px solid transparent;
      transition: color 0.15s, border-color 0.15s;
      white-space: nowrap;
      user-select: none;
      text-transform: uppercase;
      letter-spacing: 0.06em;
    }
    .tab:hover {
      color: var(--codex-text, #c8b07a);
    }
    .tab.active {
      color: var(--codex-gold, #d4a843);
      border-bottom-color: var(--codex-gold, #d4a843);
    }
    .tab-content {
      padding: var(--sp-2, 0.5rem);
      height: calc(100vh - 22rem);
      overflow-y: auto;
      white-space: pre-wrap;
      font-size: 0.68rem;
      line-height: 1.5;
      display: none;
      color: var(--codex-text, #c8b07a);
    }
    .tab-content.active {
      display: block;
    }
    #net-log {
      background: rgba(0, 0, 0, 0.3);
    }
    .log-line {
      margin-bottom: 1px;
    }
    .log-line .timestamp {
      color: var(--codex-text-dim, #7a6840);
      font-size: 0.6rem;
    }
    .net-tx {
      color: var(--codex-red, #c44a3a);
    }
    .net-rx {
      color: var(--codex-green, #4a9a4a);
    }
    .scry-footer {
      text-align: center;
      padding: var(--sp-1, 0.25rem);
      font-size: 0.65rem;
      color: var(--codex-border, #3d2e1a);
      user-select: none;
      border-top: 1px solid var(--codex-border, #3d2e1a);
    }
  </style>
  <div class="scry-header">
    ── The Scrying Pool ──
  </div>
  <div class="tabs">
    <div class="tab active" data-tab="all">All</div>
    <div class="tab" data-tab="game">Game</div>
    <div class="tab" data-tab="protocol">Protocol</div>
    <div class="tab" data-tab="state">State</div>
    <div class="tab" data-tab="system">System</div>
    <div class="tab" data-tab="network">Network</div>
  </div>
  <pre id="log-all" class="tab-content active" aria-live="polite"></pre>
  <pre id="log-game" class="tab-content"></pre>
  <pre id="log-protocol" class="tab-content"></pre>
  <pre id="log-state" class="tab-content"></pre>
  <pre id="log-system" class="tab-content"></pre>
  <pre id="net-log" class="tab-content"></pre>
  <div class="scry-footer">════════════════════════════════════════</div>
`;

class GromnieLogViewer extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));

    this._logEls = {
      all: this.shadowRoot.getElementById("log-all"),
      game: this.shadowRoot.getElementById("log-game"),
      protocol: this.shadowRoot.getElementById("log-protocol"),
      state: this.shadowRoot.getElementById("log-state"),
      system: this.shadowRoot.getElementById("log-system"),
    };
    this._netLogEl = this.shadowRoot.getElementById("net-log");

    this.shadowRoot.querySelectorAll(".tab").forEach((tab) => {
      tab.addEventListener("click", () => {
        this.shadowRoot
          .querySelectorAll(".tab")
          .forEach((t) => t.classList.remove("active"));
        this.shadowRoot
          .querySelectorAll(".tab-content")
          .forEach((c) => c.classList.remove("active"));
        tab.classList.add("active");
        const targetId =
          tab.dataset.tab === "network"
            ? "net-log"
            : `log-${tab.dataset.tab}`;
        const target = this.shadowRoot.getElementById(targetId);
        if (target) target.classList.add("active");
      });
    });
  }

  log(message) {
    this._appendLog(this._logEls.all, message);
  }

  logEvent(eventDesc) {
    this._appendLog(this._logEls.all, `event: ${eventDesc}`);

    const typeMatch = eventDesc.match(/^(\w+):/);
    if (typeMatch) {
      const type = typeMatch[1].toLowerCase();
      if (this._logEls[type]) {
        this._appendLog(this._logEls[type], eventDesc);
      }
    }
  }

  logNet(entry) {
    const line = document.createElement("div");
    line.className = entry.startsWith("[TX]") ? "net-tx" : "net-rx";
    line.textContent = entry;
    this._netLogEl.appendChild(line);
    while (this._netLogEl.children.length > 200) {
      this._netLogEl.removeChild(this._netLogEl.firstChild);
    }
    this._netLogEl.scrollTop = this._netLogEl.scrollHeight;
  }

  _appendLog(pre, message) {
    const now = new Date();
    const ts = now.toISOString().slice(11, 23);
    const line = document.createElement("div");
    line.className = "log-line";

    const tsSpan = document.createElement("span");
    tsSpan.className = "timestamp";
    tsSpan.textContent = `[${ts}] `;

    const text = document.createTextNode(message);

    line.appendChild(tsSpan);
    line.appendChild(text);
    pre.appendChild(line);

    while (pre.children.length > 500) {
      pre.removeChild(pre.firstChild);
    }
    pre.scrollTop = pre.scrollHeight;
  }
}

export { GromnieLogViewer };
