import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import { visualizer } from "rollup-plugin-visualizer";

const enableBundleReport = process.env.SPIRE_BUNDLE_REPORT === "1";

export default defineConfig({
  plugins: [
    sveltekit(),
    ...(enableBundleReport
      ? [
          visualizer({
            filename: "stats.html",
            template: "treemap",
            gzipSize: true,
            brotliSize: true,
          }),
        ]
      : []),
  ],
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
      output: {
        manualChunks(id): string | undefined {
          if (id.includes("node_modules/three")) return "vendor-three";
          if (id.includes("node_modules/chart.js")) return "vendor-chart";
          if (id.includes("node_modules/katex")) return "vendor-katex";
          return undefined;
        },
      },
    },
  },
});
