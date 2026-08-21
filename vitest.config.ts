import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    include: ["src/**/*.test.ts"],
    exclude: ["node_modules", "warp-site/**", ".svelte-kit/**", "src-tauri/target/**"],
    environment: "node",
  },
});
