// Barrel file — registers all web components.
// This file self-accepts HMR so component edits don't cause page reloads.
// define() calls are try-caught so re-execution during HMR is safe.

import { GromnieStatusBar } from "./status-bar.js";
import { GromnieLoginForm } from "./login-form.js";
import { GromnieCharacterSelect } from "./character-select.js";
import { GromnieWorldView } from "./world-view.js";
import { GromnieLogViewer } from "./log-viewer.js";
import { GromnieErrorOverlay } from "./error-overlay.js";

const components = [
  ["gromnie-status-bar", GromnieStatusBar],
  ["gromnie-login-form", GromnieLoginForm],
  ["gromnie-character-select", GromnieCharacterSelect],
  ["gromnie-world-view", GromnieWorldView],
  ["gromnie-log-viewer", GromnieLogViewer],
  ["gromnie-error-overlay", GromnieErrorOverlay],
];

for (const [name, cls] of components) {
  try {
    customElements.define(name, cls);
  } catch {}
}

if (import.meta.hot) {
  import.meta.hot.accept();
}
