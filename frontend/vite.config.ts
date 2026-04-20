import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: "0.0.0.0",
    port: 3000,
  },
  preview: {
    port: 4173,
  },
  build: {
    target: "es2022",
  },
});

