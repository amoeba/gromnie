// <gromnie-error-overlay> — "The Dark Omen" — Modal error dialog with Ultima framing.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(12, 10, 8, 0.85);
      z-index: 100;
      align-items: center;
      justify-content: center;
      backdrop-filter: blur(2px);
    }
    :host(.active) {
      display: flex;
    }
    #omen-box {
      background: var(--codex-panel, #14110d);
      border: 1px solid var(--codex-red, #c44a3a);
      border-radius: 2px;
      padding: 0;
      max-width: 24rem;
      text-align: center;
      box-shadow: 0 0 30px rgba(196, 74, 58, 0.15);
    }
    .omen-header {
      background: rgba(196, 74, 58, 0.1);
      padding: var(--sp-2, 0.5rem) var(--sp-4, 1rem);
      border-bottom: 1px solid var(--codex-red, #c44a3a);
    }
    .omen-header h3 {
      margin: 0;
      font-family: var(--font-display, serif);
      color: var(--codex-red, #c44a3a);
      font-size: 1.1rem;
      letter-spacing: 0.05em;
    }
    .omen-body {
      padding: var(--sp-4, 1rem);
    }
    .omen-icon {
      font-size: 2rem;
      margin-bottom: var(--sp-2, 0.5rem);
      color: var(--codex-red, #c44a3a);
      user-select: none;
    }
    #error-message {
      margin: 0 0 var(--sp-4, 1rem);
      color: var(--codex-text, #c8b07a);
      font-size: 0.85rem;
      line-height: 1.5;
    }
    .omen-footer {
      padding: 0 var(--sp-4, 1rem) var(--sp-4, 1rem);
    }
  </style>
  <div id="omen-box">
    <div class="omen-header">
      <h3>☠ The Dark Omen ☠</h3>
    </div>
    <div class="omen-body">
      <div class="omen-icon">⚠</div>
      <p id="error-message"></p>
    </div>
    <div class="omen-footer">
      <button class="codex-btn codex-btn--danger" id="error-ok">
        Acknowledge
      </button>
    </div>
  </div>
`;

class GromnieErrorOverlay extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));

    this._messageEl = this.shadowRoot.getElementById("error-message");
    this._okBtn = this.shadowRoot.getElementById("error-ok");

    this._okBtn.addEventListener("click", () => {
      this.hide();
      this.dispatchEvent(
        new CustomEvent("gromnie:dismiss-error", {
          bubbles: true,
          composed: true,
        })
      );
    });
  }

  show(message) {
    this._messageEl.textContent = message;
    this.classList.add("active");
  }

  hide() {
    this.classList.remove("active");
  }
}

export { GromnieErrorOverlay };
