// <gromnie-login-form> — "The Scroll of Entry" — Account view with medieval framing.

const STORAGE_KEY = "gromnie-form";

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    .scroll-header {
      text-align: center;
      margin-bottom: var(--sp-4, 1rem);
    }
    .scroll-header h2 {
      font-family: var(--font-display, serif);
      color: var(--codex-heading, #d4a843);
      font-size: 1.2rem;
      margin: 0;
      letter-spacing: 0.05em;
    }
    .scroll-header .subtitle {
      font-size: 0.7rem;
      color: var(--codex-text-dim, #7a6840);
      font-style: italic;
    }
    .scroll-header .rule {
      color: var(--codex-border, #3d2e1a);
      font-size: 0.75rem;
      user-select: none;
      margin-top: var(--sp-2, 0.5rem);
    }
    .form-grid {
      display: grid;
      grid-template-columns: 6rem 1fr;
      gap: var(--sp-2, 0.5rem);
      align-items: center;
      max-width: 32rem;
      margin: 0 auto;
    }
    .form-label {
      color: var(--codex-text-dim, #7a6840);
      font-size: 0.8rem;
      text-align: right;
      padding-right: var(--sp-2, 0.5rem);
    }
    .form-label::before {
      content: '» ';
      color: var(--codex-border, #3d2e1a);
    }
    .input-wrapper {
      position: relative;
    }
    .input-wrapper::after {
      content: attr(data-char);
      position: absolute;
      right: 0.5rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--codex-border, #3d2e1a);
      font-size: 0.8rem;
      pointer-events: none;
      opacity: 0;
    }
    .actions {
      display: flex;
      gap: var(--sp-2, 0.5rem);
      justify-content: center;
      margin-top: var(--sp-4, 1rem);
    }
  </style>

  <div class="scroll-header">
    <h2>⚜ The Scroll of Entry ⚜</h2>
    <div class="subtitle">Inscribe thy credentials to breach the gateway</div>
    <div class="rule">─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─</div>
  </div>

  <div class="form-grid">
    <label class="form-label" for="host">Host</label>
    <input class="codex-input" id="host" placeholder="play.coldeve.ac" />

    <label class="form-label" for="port">Port</label>
    <input class="codex-input" id="port" value="9000" placeholder="9000" />

    <label class="form-label" for="account">Account</label>
    <input class="codex-input" id="account" placeholder="thy name" />

    <label class="form-label" for="password">Password</label>
    <input class="codex-input" id="password" type="password" placeholder="thy secret word" />
  </div>

  <div class="actions">
    <button class="codex-btn codex-btn--primary" id="login" disabled>
      ⚔ Enter the Gateway
    </button>
    <button class="codex-btn" id="reset-form" type="button">
      ↺ Clear
    </button>
  </div>
`;

class GromnieLoginForm extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));

    this._hostEl = this.shadowRoot.getElementById("host");
    this._portEl = this.shadowRoot.getElementById("port");
    this._accountEl = this.shadowRoot.getElementById("account");
    this._passwordEl = this.shadowRoot.getElementById("password");
    this._loginBtn = this.shadowRoot.getElementById("login");
    this._resetBtn = this.shadowRoot.getElementById("reset-form");

    this._loginBtn.addEventListener("click", () => this._doLogin());
    this._resetBtn.addEventListener("click", () => this._doReset());
    [this._hostEl, this._portEl, this._accountEl].forEach((el) =>
      el.addEventListener("input", () => this._saveForm())
    );

    this._loadForm();
  }

  setLoginEnabled(enabled) {
    this._loginBtn.disabled = !enabled;
  }

  _saveForm() {
    const data = {
      host: this._hostEl.value,
      port: this._portEl.value,
      account: this._accountEl.value,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
  }

  _loadForm() {
    try {
      const data = JSON.parse(localStorage.getItem(STORAGE_KEY));
      if (data) {
        if (data.host) this._hostEl.value = data.host;
        if (data.port) this._portEl.value = data.port;
        if (data.account) this._accountEl.value = data.account;
      }
    } catch {}
  }

  _doLogin() {
    this.dispatchEvent(
      new CustomEvent("gromnie:connect", {
        detail: {
          host: this._hostEl.value.trim(),
          port: parseInt(this._portEl.value.trim(), 10),
          account: this._accountEl.value.trim(),
          password: this._passwordEl.value.trim(),
        },
        bubbles: true,
        composed: true,
      })
    );
  }

  _doReset() {
    this._hostEl.value = "play.coldeve.ac";
    this._portEl.value = "9000";
    this._accountEl.value = "";
    this._passwordEl.value = "";
    this._saveForm();
    this.dispatchEvent(
      new CustomEvent("gromnie:reset", {
        bubbles: true,
        composed: true,
      })
    );
  }
}

export { GromnieLoginForm };
