// <gromnie-log-viewer> — Tabbed log panels (All/Game/Protocol/State/System/Network).

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
      border: 1px solid #ddd;
      padding: 0.5rem;
      border-radius: 4px;
    }
    .tabs {
      display: flex;
      gap: 0;
      margin-bottom: 0;
    }
    .tab {
      padding: 0.25rem 0.6rem;
      border: 1px solid #ccc;
      border-bottom: none;
      cursor: pointer;
      font-size: 0.75rem;
      background: #f5f5f5;
      border-radius: 3px 3px 0 0;
    }
    .tab.active {
      background: #fff;
      border-bottom: 1px solid #fff;
      margin-bottom: -1px;
    }
    .tab-content {
      border: 1px solid #ccc;
      padding: 0.3rem;
      height: calc(100vh - 14rem);
      overflow-y: auto;
      white-space: pre-wrap;
      font-size: 0.7rem;
      display: none;
    }
    .tab-content.active {
      display: block;
    }
    #net-log {
      background: #111;
      color: #0f0;
    }
  </style>
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
    const color = entry.startsWith("[TX]") ? "#f88" : "#8f8";
    const line = document.createElement("div");
    line.style.color = color;
    line.textContent = entry;
    this._netLogEl.appendChild(line);
    while (this._netLogEl.children.length > 200) {
      this._netLogEl.removeChild(this._netLogEl.firstChild);
    }
    this._netLogEl.scrollTop = this._netLogEl.scrollHeight;
  }

  _appendLog(pre, message) {
    const line = `[${new Date().toISOString()}] ${message}`;
    pre.textContent += `${line}\n`;
    pre.scrollTop = pre.scrollHeight;
  }
}

export { GromnieLogViewer };
