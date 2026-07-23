// <gromnie-login-form> — Account view with host/port/account/password fields.

const STORAGE_KEY = "gromnie-form";

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    h2 {
      margin-top: 0;
    }
    .row {
      display: grid;
      grid-template-columns: 4.5rem 1fr;
      gap: 0.25rem;
      margin-bottom: 0.25rem;
      align-items: center;
    }
    input {
      width: 100%;
      padding: 0.2rem 0.3rem;
      font-size: 0.85rem;
      font-family: inherit;
    }
    .actions {
      display: flex;
      gap: 0.4rem;
      margin-top: 0.4rem;
    }
    button {
      padding: 0.25rem 0.5rem;
      font-size: 0.85rem;
      font-family: inherit;
    }
  </style>
  <h2>Account Info</h2>
  <div class="row">
    <label for="host">host</label>
    <input id="host" />
  </div>
  <div class="row">
    <label for="port">port</label>
    <input id="port" value="9000" />
  </div>
  <div class="row">
    <label for="account">account</label>
    <input id="account" />
  </div>
  <div class="row">
    <label for="password">password</label>
    <input id="password" type="password" />
  </div>
  <div class="actions">
    <button id="login" disabled>Login</button>
    <button id="reset-form" type="button">Reset</button>
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
