import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Puerto fijo: tauri.conf.json apunta a él en desarrollo.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
