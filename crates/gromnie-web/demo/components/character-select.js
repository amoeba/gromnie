// <gromnie-character-select> — "The Hall of Heroes" — Character list with Ultima framing.

const TEMPLATE = document.createElement("template");
TEMPLATE.innerHTML = `
  <style>
    :host {
      display: block;
    }
    .hall-header {
      text-align: center;
      margin-bottom: var(--sp-3, 0.75rem);
    }
    .hall-header h2 {
      font-family: var(--font-display, serif);
      color: var(--codex-heading, #d4a843);
      font-size: 1.2rem;
      margin: 0;
    }
    .hall-header .subtitle {
      font-size: 0.7rem;
      color: var(--codex-text-dim, #7a6840);
      font-style: italic;
    }
    .hall-header .rule {
      color: var(--codex-border, #3d2e1a);
      font-size: 0.75rem;
      user-select: none;
      margin-top: var(--sp-2, 0.5rem);
    }
    #char-list {
      border: 1px solid var(--codex-border, #3d2e1a);
      background: var(--codex-bg, #0c0a08);
      min-height: 6rem;
      max-height: 16rem;
      overflow-y: auto;
      margin-bottom: var(--sp-3, 0.75rem);
    }
    .char-row {
      display: flex;
      align-items: center;
      padding: 0.35rem 0.6rem;
      cursor: pointer;
      border-bottom: 1px solid rgba(61, 46, 26, 0.3);
      transition: background 0.1s;
      font-size: 0.85rem;
    }
    .char-row:last-child {
      border-bottom: none;
    }
    .char-row:hover {
      background: rgba(212, 168, 67, 0.05);
    }
    .char-row.selected {
      background: rgba(212, 168, 67, 0.1);
      border-left: 2px solid var(--codex-gold, #d4a843);
      padding-left: calc(0.6rem - 2px);
    }
    .char-marker {
      color: var(--codex-border, #3d2e1a);
      margin-right: 0.4rem;
      user-select: none;
      font-size: 0.75rem;
    }
    .char-row.selected .char-marker {
      color: var(--codex-gold, #d4a843);
    }
    .char-name {
      color: var(--codex-text, #c8b07a);
    }
    .char-row.selected .char-name {
      color: var(--codex-text-bright, #e8d4a0);
    }
    .empty-msg {
      padding: 1.5rem;
      text-align: center;
      color: var(--codex-text-dim, #7a6840);
      font-style: italic;
      font-size: 0.8rem;
    }
    .actions {
      display: flex;
      gap: var(--sp-2, 0.5rem);
      justify-content: center;
    }
  </style>

  <div class="hall-header">
    <h2>⚜ The Hall of Heroes ⚜</h2>
    <div class="subtitle">Choose thy champion and enter the realm</div>
    <div class="rule">─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─</div>
  </div>

  <div id="char-list">
    <div class="empty-msg">Awaiting thy summons...</div>
  </div>

  <div class="actions">
    <button class="codex-btn codex-btn--primary" id="enter-world" disabled>
      ⚔ Enter the Realm
    </button>
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
    if (this._characters.length === 0) {
      this._charListEl.innerHTML = `<div class="empty-msg">Awaiting thy summons...</div>`;
      return;
    }
    this._characters.forEach((c) => {
      const row = document.createElement("div");
      row.className = "char-row";
      if (c.id === this._selectedId) row.classList.add("selected");

      const marker = document.createElement("span");
      marker.className = "char-marker";
      marker.textContent = c.id === this._selectedId ? "►" : " ";

      const name = document.createElement("span");
      name.className = "char-name";
      name.textContent = c.name;

      row.appendChild(marker);
      row.appendChild(name);

      row.addEventListener("click", () => {
        this._selectedId = c.id;
        this._enterBtn.disabled = false;
        this._render();
      });

      this._charListEl.appendChild(row);
    });

    if (this._selectedId !== null) {
      this._enterBtn.disabled = false;
    }
  }
}

export { GromnieCharacterSelect };
