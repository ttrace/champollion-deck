import { defineConfig } from "vite";

export default defineConfig({
  clearScreen: false,
  server: {
    strictPort: true,
    port: 1420
  },
  build: {
    target: "es2020",
    outDir: "dist",
    emptyOutDir: true
  }
});
