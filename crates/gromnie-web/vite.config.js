import { defineConfig } from "vite";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: path.resolve(__dirname, "demo"),
  publicDir: "public",
  server: {
    port: 5173,
    fs: {
      // Allow serving files from parent directory (demo/pkg symlinks to ../pkg)
      allow: [path.resolve(__dirname, "demo"), path.resolve(__dirname)],
    },
    proxy: {
      // Proxy WISP WebSocket connections to gromnie-proxy
      "/wisp": {
        target: "ws://127.0.0.1:8081",
        ws: true,
      },
      "/auth": {
        target: "http://127.0.0.1:8081",
      },
    },
  },
  watch: {
    // Avoid following symlinks into pkg/ (prevents symlink loops)
    followSymlinks: false,
  },
});
