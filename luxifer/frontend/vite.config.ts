import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri erwartet einen festen Dev-Port und lauscht selbst auf Änderungen.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // Tauri: sauberer, vorhersehbarer Dev-Server.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 5183 }
      : undefined,
    watch: {
      // Das Rust-Backend nicht vom Vite-Watcher beobachten lassen.
      ignored: ["**/src-tauri/**"],
    },
  },
});
