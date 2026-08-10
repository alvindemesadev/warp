// Preview build for the marketing site — compiles the REAL app UI
// (+page.svelte + app.css) into a self-contained bundle the site embeds in an
// iframe. Tauri imports are aliased to preview-shims/ so the component renders
// in a plain browser. Run: npm run preview:build
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const shim = (name: string) => path.resolve(__dirname, `src/preview-shims/${name}.ts`);

const conf = JSON.parse(fs.readFileSync(path.resolve(__dirname, "src-tauri/tauri.conf.json"), "utf8"));

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, "src/lib"),
      "@tauri-apps/api/core": shim("core"),
      "@tauri-apps/api/event": shim("event"),
      "@tauri-apps/api/window": shim("window"),
      "@tauri-apps/api/app": shim("app"),
      "@tauri-apps/plugin-dialog": shim("plugin-dialog"),
      "@tauri-apps/plugin-notification": shim("plugin-notification"),
      "@tauri-apps/plugin-updater": shim("plugin-updater"),
    },
  },
  define: {
    __PREVIEW_VERSION__: JSON.stringify(conf.version),
  },
  publicDir: false,
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/site-preview.ts"),
      name: "WarpPreview",
      formats: ["iife"],
      fileName: () => "site-preview.js",
      cssFileName: "site-preview",
    },
    outDir: "site-preview-dist",
    cssCodeSplit: false,
    target: "es2020",
  },
});
