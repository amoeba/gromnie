// <gromnie-character-select> — Character list and enter world button.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    h2 {
      margin-top: 0;
    }
    #char-list {
      font-size: 0.8rem;
      color: #666;
    }
    #char-list div {
      padding: 0.3rem 0.5rem;
      cursor: pointer;
      border: 1px solid transparent;
      border-radius: 3px;
      margin-bottom: 0.15rem;
    }
    #char-list div:hover {
      background: #eef !important;
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
  <h2>Character Select</h2>
  <div id="char-list"></div>
  <div class="actions">
    <button id="enter-world" disabled>Enter World</button>
  </div>
`;

class GromnieCharacterSelect extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.shadowRoot.appendChild(TEMPLATE.content.cloneNode(true));

    this._charListEl = this.shadowRoot.getElementById("char-list");
    this._enterBtn = this.shadowRoot.getElementById("enter-world");

    this._characters = [];
    this._selectedId = null;

    this._enterBtn.addEventListener("click", () => {
      if (this._selectedId !== null) {
        this.dispatchEvent(
          new CustomEvent("gromnie:select-character", {
            detail: { characterId: this._selectedId },
            bubbles: true,
            composed: true,
          })
        );
      }
    });
  }

  setCharacters(characters) {
    this._characters = characters;
    this._render();
  }

  setSelectedId(id) {
    this._selectedId = id;
    this._render();
  }

  setEnterEnabled(enabled) {
    this._enterBtn.disabled = !enabled;
  }

  _render() {
    this._charListEl.innerHTML = "";
    this._characters.forEach((c, i) => {
      const row = document.createElement("div");
      row.textContent = c.name;
      row.addEventListener("click", () => {
        this._charListEl
          .querySelectorAll("div")
          .forEach((r) => (r.style.background = ""));
        row.style.background = "#dde";
        this._selectedId = c.id;
        this._enterBtn.disabled = false;
      });
      if (i === 0 || c.id === this._selectedId) {
        row.style.background = "#dde";
        this._selectedId = c.id;
        this._enterBtn.disabled = false;
      }
      this._charListEl.appendChild(row);
    });
  }
}

export { GromnieCharacterSelect };
