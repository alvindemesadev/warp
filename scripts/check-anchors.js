#!/usr/bin/env node
// check-anchors.js — lints `file_path:line` anchors in docs/*.md and ROADMAP.md.
// Fails if a referenced file doesn't exist or line is out of range.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const docs = [
  "ROADMAP.md",
  "docs/WHITEPAPER.md",
  "docs/ARCHITECTURE.md",
  "docs/roadmap/PHASE-01-code-health.md",
  "docs/roadmap/PHASE-02-architecture.md",
  "docs/roadmap/PHASE-03-testing.md",
  "docs/roadmap/PHASE-04-security.md",
  "docs/roadmap/PHASE-05-performance-ux.md",
  "docs/roadmap/PHASE-06-ci-cd.md",
  "docs/roadmap/PHASE-07-docs-dx.md",
  "docs/roadmap/PHASE-08-future.md",
];

let failed = false;
const re = /([a-zA-Z0-9_\-./]+\.(?:rs|ts|svelte|js|json|md|toml)):(\d+)/g;

function findByBasename(basename) {
  const stack = [ROOT];
  const alts = [basename];
  if (basename === "page.svelte") alts.push("+page.svelte");
  while (stack.length) {
    const dir = stack.pop();
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const e of entries) {
      if (
        e.name === "node_modules" ||
        e.name === ".git" ||
        e.name === "target" ||
        e.name === ".svelte-kit"
      )
        continue;
      const full = path.join(dir, e.name);
      if (e.isDirectory()) stack.push(full);
      else if (alts.includes(e.name)) return full;
    }
  }
  return null;
}

for (const rel of docs) {
  const p = path.join(ROOT, rel);
  if (!fs.existsSync(p)) continue;
  const content = fs.readFileSync(p, "utf8");
  let m;
  while ((m = re.exec(content)) !== null) {
    const file = m[1];
    const line = parseInt(m[2], 10);
    let full = path.join(ROOT, file);
    if (!fs.existsSync(full)) {
      // Try shorthand: e.g. "shards.rs:48" or "WHITEPAPER.md:361" or "tauri.conf.json:6"
      const base = path.basename(file);
      const found = findByBasename(base);
      if (found) full = found;
      else {
        console.error(`✖ ${rel}: ${file}:${line} — file not found`);
        failed = true;
        continue;
      }
    }
    const lines = fs.readFileSync(full, "utf8").split("\n").length;
    if (line < 1 || line > lines + 50) {
      console.error(`✖ ${rel}: ${file}:${line} — line out of range (1..${lines})`);
      failed = true;
    }
  }
}

if (failed) {
  console.error("\nAnchor check failed — fix file_path:line refs.");
  process.exit(1);
}
console.log("✔ anchors ok");
