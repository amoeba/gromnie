// Stable entry point — creates the SharedWorker and initializes the UI.
// This file should NEVER be edited during UI development.
// All UI logic lives in ui.js and is hot-reloadable.

import { init, handleMessage } from "./ui.js";

const worker = new SharedWorker("/worker.js", {
  type: "module",
  name: "gromnie",
});
const port = worker.port;
port.start();

let currentHandleMessage = handleMessage;
port.onmessage = (e) => currentHandleMessage(e.data);

init(port);

if (import.meta.hot) {
  import.meta.hot.accept("./ui.js", (newModule) => {
    if (newModule) {
      currentHandleMessage = newModule.handleMessage;
    }
  });
}
