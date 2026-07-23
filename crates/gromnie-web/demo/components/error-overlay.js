// <gromnie-error-overlay> — Modal error dialog.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(0,0,0,0.5);
      z-index: 100;
      align-items: center;
      justify-content: center;
    }
    :host(.active) {
      display: flex;
    }
    #error-box {
      background: #fff;
      border: 2px solid #c44;
      border-radius: 6px;
      padding: 1.2rem;
      max-width: 20rem;
      text-align: center;
    }
    #error-box h3 {
      margin: 0 0 0.5rem;
      color: #c44;
    }
    #error-box p {
      margin: 0 0 0.8rem;
    }
    button {
      padding: 0.25rem 0.5rem;
      font-size: 0.85rem;
      font-family: inherit;
    }
  </style>
  <div id="error-box">
    <h3>Login Failed</h3>
    <p id="error-message"></p>
    <button id="error-ok">OK</button>
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
