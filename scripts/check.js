#!/usr/bin/env node
// Wrapper for svelte-check that filters the known warp-site/workspace noise
// (warp-site is a separate React Vite project, not Svelte, but lives inside the monorepo as a gitignored checkout).
import { spawnSync } from "node:child_process";

const args = ["svelte-check", "--tsconfig", "./tsconfig.json"];
const res = spawnSync("npx", args, { stdio: "pipe", encoding: "utf8", shell: true });

// Filter known noise, keep real diagnostics
let out = (res.stdout || "") + (res.stderr || "");
const lines = out.split("\n");
const filtered = lines.filter((l) => {
  if (l.includes("warp-site/vite.config.ts")) return false;
  if (l.includes("No Svelte configuration found in vite config")) return false;
  if (l.includes("Error while loading config at")) return false;
  if (l.includes("ConfigLoader.loadConfig")) return false;
  // Vite override warning is not an error, just info
  if (l.includes("Vite config options will be overridden")) return false;
  if (l.includes("The following Vite config options will be overridden")) return false;
  return true;
});

// Heuristic: svelte-check prints "found X errors and Y warnings" — treat any non-zero as failure.
// Warp-site noise is filtered above, so only real diagnostics remain.
let hasRealError = false;
for (const l of filtered) {
  const m = l.match(/found\s+(\d+)\s+errors?\s+and\s+(\d+)\s+warnings?/i);
  if (m) {
    const errs = parseInt(m[1] ?? "0", 10);
    const warns = parseInt(m[2] ?? "0", 10);
    if (errs !== 0 || warns !== 0) hasRealError = true;
    break;
  }
  // Fallback: generic "found X errors" without warnings part
  if (l.includes("found") && l.includes("errors") && !l.includes("0 errors")) {
    hasRealError = true;
    break;
  }
}
const filteredOut = filtered.join("\n");
process.stdout.write(filteredOut);
if (hasRealError) process.exit(1);
else process.exit(0);
