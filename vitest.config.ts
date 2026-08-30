import { defineConfig } from "vitest/config";
import { sveltekit } from "@sveltejs/kit/vite";

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    include: ["src/**/*.test.ts"],
    exclude: ["node_modules", "warp-site/**", ".svelte-kit/**", "src-tauri/target/**"],
    environment: "node",
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      include: ["src/lib/**/*.{ts,svelte}"],
      exclude: [
        "src/lib/**/*.test.ts",
        "src/lib/stores/**/*.svelte.ts",
        "src/lib/components/**",
        "src/lib/services/warp.ts",
        "src/lib/tauri.ts",
        "src/lib/types.ts",
        "src-tauri/**",
        "warp-site/**",
      ],
      thresholds: {
        // Phase 3: 77% now, target 95%/90% after E2E (see PHASE-03)
        lines: 70,
        branches: 60,
        functions: 70,
        statements: 70,
      },
    },
  },
});
