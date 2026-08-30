import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

export default tseslint.config(
  {
    ignores: [
      "build/**",
      ".svelte-kit/**",
      "src-tauri/target/**",
      "node_modules/**",
      "warp-site/**",
      "static/**",
      "docs/*.exe",
      "dist/**",
      "coverage/**",
      "test-results/**",
      "playwright-report/**",
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    rules: {
      // No anys without thought — but allow explicit `any` with comment
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
      "no-unused-vars": "off",
      // Allow empty catch `catch {}` (intentional swallows in storage.ts etc.)
      "no-empty": ["error", { allowEmptyCatch: true }],
      // Svelte specific — keep useful a11y
      "svelte/no-at-html-tags": "error",
      // Each keys are nice-to-have, not blockers for static lists
      "svelte/require-each-key": "off",
    },
  },
  {
    // scripts/*.js are plain JS with node globals
    files: ["scripts/**/*.js"],
    rules: {
      "@typescript-eslint/no-require-imports": "off",
      "no-useless-escape": "off",
      "no-empty": ["error", { allowEmptyCatch: true }],
    },
  },
  {
    // Tests intentionally use `as any` to probe invalid inputs
    files: ["**/*.test.ts", "**/*.test.js"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
  {
    // Svelte runes stores use @ts-nocheck to defer $state/$derived to svelte-check
    files: ["**/*.svelte.ts", "**/*.svelte.js"],
    rules: {
      "@typescript-eslint/ban-ts-comment": "off",
    },
  },
);
