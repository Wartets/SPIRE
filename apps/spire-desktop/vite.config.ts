import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [sveltekit()],
  worker: {
    format: "es",
    rollupOptions: {
      // spire-kernel-wasm is an optional WASM artifact built separately.
      // Workers that import it use try/catch for graceful fallback.
      external: ["spire-kernel-wasm"],
    },
  },
  build: {
    rollupOptions: {
      external: ["spire-kernel-wasm"],
    },
  },
});
